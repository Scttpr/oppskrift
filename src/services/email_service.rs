//! Email service for sending authentication-related emails
//!
//! Uses lettre for SMTP delivery. If SMTP is not configured,
//! emails are logged instead (useful for development).

use crate::lib::config::SmtpConfig;
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Failed to build email: {0}")]
    BuildError(String),
    #[error("Failed to send email: {0}")]
    SendError(String),
}

/// Email service for sending authentication emails
#[derive(Clone)]
pub struct EmailService {
    config: Option<SmtpConfig>,
    base_url: String,
}

impl EmailService {
    /// Create a new email service
    ///
    /// If config is None, emails will be logged instead of sent.
    pub fn new(config: Option<SmtpConfig>, base_url: String) -> Self {
        Self { config, base_url }
    }

    /// Check if email sending is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.is_some()
    }

    /// Send an email
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        let config = match &self.config {
            Some(c) => c,
            None => {
                // Log email instead of sending (development mode)
                tracing::info!(
                    to = to,
                    subject = subject,
                    "Email not sent (SMTP not configured):\n{}",
                    body
                );
                return Ok(());
            }
        };

        let from: Mailbox = format!("{} <{}>", config.from_name, config.from_address)
            .parse()
            .map_err(|e| EmailError::BuildError(format!("Invalid from address: {}", e)))?;

        let to_mailbox: Mailbox = to
            .parse()
            .map_err(|e| EmailError::BuildError(format!("Invalid to address: {}", e)))?;

        let email = Message::builder()
            .from(from)
            .to(to_mailbox)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
            .map_err(|e| EmailError::BuildError(e.to_string()))?;

        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let mailer: AsyncSmtpTransport<Tokio1Executor> =
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                .map_err(|e| EmailError::SendError(e.to_string()))?
                .credentials(creds)
                .port(config.port)
                .build();

        mailer
            .send(email)
            .await
            .map_err(|e| EmailError::SendError(e.to_string()))?;

        tracing::info!(to = to, subject = subject, "Email sent successfully");
        Ok(())
    }

    /// Send email confirmation link
    pub async fn send_confirmation(&self, to: &str, token: &str) -> Result<(), EmailError> {
        let link = format!("{}/api/auth/confirm-email/{}", self.base_url, token);

        let subject = "Confirm your Oppskrift account";
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Confirm your email</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Welcome to Oppskrift!</h1>
    <p>Please confirm your email address by clicking the button below:</p>
    <p style="margin: 30px 0;">
        <a href="{link}" style="background-color: #2563eb; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; display: inline-block;">
            Confirm Email
        </a>
    </p>
    <p style="color: #666; font-size: 14px;">
        Or copy and paste this link into your browser:<br>
        <a href="{link}" style="color: #2563eb;">{link}</a>
    </p>
    <p style="color: #666; font-size: 14px;">
        This link will expire in 24 hours.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        If you didn't create an account on Oppskrift, you can safely ignore this email.
    </p>
</body>
</html>"#,
            link = link
        );

        self.send(to, subject, &body).await
    }

    /// Send password reset link
    pub async fn send_password_reset(&self, to: &str, token: &str) -> Result<(), EmailError> {
        let link = format!("{}/reset-password?token={}", self.base_url, token);

        let subject = "Reset your Oppskrift password";
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Reset your password</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Reset your password</h1>
    <p>We received a request to reset your Oppskrift password. Click the button below to choose a new password:</p>
    <p style="margin: 30px 0;">
        <a href="{link}" style="background-color: #2563eb; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; display: inline-block;">
            Reset Password
        </a>
    </p>
    <p style="color: #666; font-size: 14px;">
        Or copy and paste this link into your browser:<br>
        <a href="{link}" style="color: #2563eb;">{link}</a>
    </p>
    <p style="color: #666; font-size: 14px;">
        This link will expire in 1 hour.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        If you didn't request a password reset, you can safely ignore this email.
        Your password will not be changed.
    </p>
</body>
</html>"#,
            link = link
        );

        self.send(to, subject, &body).await
    }

    /// Send password changed notification
    pub async fn send_password_changed_notification(&self, to: &str) -> Result<(), EmailError> {
        let subject = "Your Oppskrift password was changed";
        let body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Password changed</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Password Changed</h1>
    <p>Your Oppskrift password was recently changed.</p>
    <p>If you made this change, you can safely ignore this email.</p>
    <p style="color: #dc2626; font-weight: bold;">
        If you did not change your password, please secure your account immediately
        by resetting your password and reviewing your active sessions.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        This is an automated security notification from Oppskrift.
    </p>
</body>
</html>"#;

        self.send(to, subject, body).await
    }

    /// Send 2FA enabled notification
    pub async fn send_2fa_enabled_notification(&self, to: &str) -> Result<(), EmailError> {
        let subject = "Two-factor authentication enabled on Oppskrift";
        let body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>2FA Enabled</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Two-Factor Authentication Enabled</h1>
    <p>Two-factor authentication has been enabled on your Oppskrift account.</p>
    <p>From now on, you'll need to enter a code from your authenticator app when logging in.</p>
    <p style="color: #059669; font-weight: bold;">
        Make sure you've saved your recovery codes in a safe place.
        You'll need them if you lose access to your authenticator app.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        If you did not enable 2FA, please secure your account immediately.
    </p>
</body>
</html>"#;

        self.send(to, subject, body).await
    }

    /// Send 2FA disabled notification
    pub async fn send_2fa_disabled_notification(&self, to: &str) -> Result<(), EmailError> {
        let subject = "Two-factor authentication disabled on Oppskrift";
        let body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>2FA Disabled</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Two-Factor Authentication Disabled</h1>
    <p>Two-factor authentication has been disabled on your Oppskrift account.</p>
    <p style="color: #dc2626; font-weight: bold;">
        Your account is now less secure. We recommend re-enabling 2FA for better protection.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        If you did not disable 2FA, please secure your account immediately.
    </p>
</body>
</html>"#;

        self.send(to, subject, body).await
    }

    /// Send account deletion scheduled notification
    pub async fn send_deletion_scheduled_notification(
        &self,
        to: &str,
        deletion_date: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), EmailError> {
        let date_str = deletion_date.format("%Y-%m-%d %H:%M UTC").to_string();
        let subject = "Your Oppskrift account deletion is scheduled";
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Account Deletion Scheduled</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Account Deletion Scheduled</h1>
    <p>Your Oppskrift account is scheduled for deletion on <strong>{}</strong>.</p>
    <p>If you change your mind, you can cancel the deletion by logging in before that date.</p>
    <p style="color: #dc2626; font-weight: bold;">
        After this date, your account and all associated data will be permanently deleted
        and cannot be recovered.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        This is an automated notification from Oppskrift.
    </p>
</body>
</html>"#,
            date_str
        );

        self.send(to, subject, &body).await
    }

    /// Send account deletion cancelled notification
    pub async fn send_deletion_cancelled_notification(&self, to: &str) -> Result<(), EmailError> {
        let subject = "Your Oppskrift account deletion has been cancelled";
        let body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Account Deletion Cancelled</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Account Deletion Cancelled</h1>
    <p>Your Oppskrift account deletion has been cancelled.</p>
    <p style="color: #059669; font-weight: bold;">
        Your account is now active again and all your data has been preserved.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        This is an automated notification from Oppskrift.
    </p>
</body>
</html>"#;

        self.send(to, subject, body).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_service_disabled() {
        let service = EmailService::new(None, "http://localhost:3000".to_string());
        assert!(!service.is_enabled());
    }

    #[test]
    fn test_email_service_enabled() {
        let config = SmtpConfig {
            host: "smtp.example.com".to_string(),
            port: 587,
            username: "user".to_string(),
            password: "pass".to_string(),
            from_address: "noreply@example.com".to_string(),
            from_name: "Test".to_string(),
        };
        let service = EmailService::new(Some(config), "http://localhost:3000".to_string());
        assert!(service.is_enabled());
    }
}
