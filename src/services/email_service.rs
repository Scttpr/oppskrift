//! Email service for sending authentication-related emails
//!
//! Uses lettre for SMTP delivery. If SMTP is not configured,
//! emails are logged instead (useful for development).

use crate::core::config::SmtpConfig;
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

        // Build SMTP transport - use plain SMTP for local dev (port 1025), TLS for production
        let mailer: AsyncSmtpTransport<Tokio1Executor> = if config.port == 1025 {
            // Local development (e.g., Mailpit) - no TLS, no auth
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
                .port(config.port)
                .build()
        } else {
            // Production - use TLS and credentials
            let creds = Credentials::new(config.username.clone(), config.password.clone());
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                .map_err(|e| EmailError::SendError(e.to_string()))?
                .credentials(creds)
                .port(config.port)
                .build()
        };

        // Bound the SMTP send so a slow/hung relay can't stall the request indefinitely.
        tokio::time::timeout(std::time::Duration::from_secs(30), mailer.send(email))
            .await
            .map_err(|_| EmailError::SendError("SMTP send timed out".to_string()))?
            .map_err(|e| EmailError::SendError(e.to_string()))?;

        tracing::info!(to = to, subject = subject, "Email sent successfully");
        Ok(())
    }

    /// Send email confirmation link
    pub async fn send_confirmation(&self, to: &str, token: &str) -> Result<(), EmailError> {
        let link = format!("{}/confirm-email/{}", self.base_url, token);

        let subject = "Confirme ton compte Oppskrift";
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Confirme ton e-mail</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Bienvenue sur Oppskrift !</h1>
    <p>Confirme ton adresse e-mail en cliquant sur le bouton ci-dessous :</p>
    <p style="margin: 30px 0;">
        <a href="{link}" style="background-color: #2563eb; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; display: inline-block;">
            Confirmer l'e-mail
        </a>
    </p>
    <p style="color: #666; font-size: 14px;">
        Ou copie-colle ce lien dans ton navigateur :<br>
        <a href="{link}" style="color: #2563eb;">{link}</a>
    </p>
    <p style="color: #666; font-size: 14px;">
        Ce lien expirera dans 24 heures.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        Si tu n'as pas créé de compte sur Oppskrift, tu peux ignorer cet e-mail.
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

        let subject = "Réinitialise ton mot de passe Oppskrift";
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Réinitialise ton mot de passe</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Réinitialise ton mot de passe</h1>
    <p>Nous avons reçu une demande de réinitialisation de ton mot de passe Oppskrift. Clique sur le bouton ci-dessous pour en choisir un nouveau :</p>
    <p style="margin: 30px 0;">
        <a href="{link}" style="background-color: #2563eb; color: white; padding: 12px 24px; text-decoration: none; border-radius: 6px; display: inline-block;">
            Réinitialiser le mot de passe
        </a>
    </p>
    <p style="color: #666; font-size: 14px;">
        Ou copie-colle ce lien dans ton navigateur :<br>
        <a href="{link}" style="color: #2563eb;">{link}</a>
    </p>
    <p style="color: #666; font-size: 14px;">
        Ce lien expirera dans 1 heure.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        Si tu n'as pas demandé de réinitialisation, tu peux ignorer cet e-mail.
        Ton mot de passe ne sera pas modifié.
    </p>
</body>
</html>"#,
            link = link
        );

        self.send(to, subject, &body).await
    }

    /// Send password changed notification
    pub async fn send_password_changed_notification(&self, to: &str) -> Result<(), EmailError> {
        let subject = "Ton mot de passe Oppskrift a été modifié";
        let body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Mot de passe modifié</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Mot de passe modifié</h1>
    <p>Ton mot de passe Oppskrift a récemment été modifié.</p>
    <p>Si tu es à l'origine de ce changement, tu peux ignorer cet e-mail.</p>
    <p style="color: #dc2626; font-weight: bold;">
        Si tu n'as pas modifié ton mot de passe, sécurise ton compte immédiatement
        en réinitialisant ton mot de passe et en vérifiant tes sessions actives.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        Ceci est une notification de sécurité automatique d'Oppskrift.
    </p>
</body>
</html>"#;

        self.send(to, subject, body).await
    }

    /// Send 2FA enabled notification
    pub async fn send_2fa_enabled_notification(&self, to: &str) -> Result<(), EmailError> {
        let subject = "Authentification à deux facteurs activée sur Oppskrift";
        let body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>2FA activée</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Authentification à deux facteurs activée</h1>
    <p>L'authentification à deux facteurs a été activée sur ton compte Oppskrift.</p>
    <p>Désormais, tu devras saisir un code de ton application d'authentification à chaque connexion.</p>
    <p style="color: #059669; font-weight: bold;">
        Assure-toi d'avoir enregistré tes codes de récupération en lieu sûr.
        Tu en auras besoin si tu perds l'accès à ton application d'authentification.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        Si tu n'as pas activé la 2FA, sécurise ton compte immédiatement.
    </p>
</body>
</html>"#;

        self.send(to, subject, body).await
    }

    /// Send 2FA disabled notification
    pub async fn send_2fa_disabled_notification(&self, to: &str) -> Result<(), EmailError> {
        let subject = "Authentification à deux facteurs désactivée sur Oppskrift";
        let body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>2FA désactivée</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Authentification à deux facteurs désactivée</h1>
    <p>L'authentification à deux facteurs a été désactivée sur ton compte Oppskrift.</p>
    <p style="color: #dc2626; font-weight: bold;">
        Ton compte est désormais moins sécurisé. Nous te recommandons de réactiver la 2FA pour une meilleure protection.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        Si tu n'as pas désactivé la 2FA, sécurise ton compte immédiatement.
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
        let date_str = crate::core::helpers::format_fr_datetime(&deletion_date);
        let subject = "La suppression de ton compte Oppskrift est programmée";
        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Suppression de compte programmée</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Suppression de compte programmée</h1>
    <p>La suppression de ton compte Oppskrift est programmée pour le <strong>{}</strong>.</p>
    <p>Si tu changes d'avis, tu peux annuler la suppression en te connectant avant cette date.</p>
    <p style="color: #dc2626; font-weight: bold;">
        Après cette date, ton compte et toutes les données associées seront définitivement supprimés
        et ne pourront pas être récupérés.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        Ceci est une notification automatique d'Oppskrift.
    </p>
</body>
</html>"#,
            date_str
        );

        self.send(to, subject, &body).await
    }

    /// Send account deletion cancelled notification
    pub async fn send_deletion_cancelled_notification(&self, to: &str) -> Result<(), EmailError> {
        let subject = "La suppression de ton compte Oppskrift a été annulée";
        let body = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Suppression de compte annulée</title>
</head>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
    <h1 style="color: #333;">Suppression de compte annulée</h1>
    <p>La suppression de ton compte Oppskrift a été annulée.</p>
    <p style="color: #059669; font-weight: bold;">
        Ton compte est de nouveau actif et toutes tes données ont été conservées.
    </p>
    <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
    <p style="color: #999; font-size: 12px;">
        Ceci est une notification automatique d'Oppskrift.
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
