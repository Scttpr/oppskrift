//! Security event logging service
//!
//! Provides audit logging for all security-relevant authentication events.
//! Events are stored in the security_events table for compliance and forensics.

use sqlx::PgPool;
use std::net::IpAddr;
use uuid::Uuid;

/// Security event types matching the database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityEventType {
    // Registration
    RegisterSuccess,
    RegisterFailure,
    // Login/Logout
    LoginSuccess,
    LoginFailure,
    LoginLocked,
    Logout,
    // Password
    PasswordResetRequest,
    PasswordResetComplete,
    PasswordChange,
    // Email
    EmailChange,
    EmailConfirmed,
    // 2FA
    TotpEnable,
    TotpDisable,
    RecoveryCodeUsed,
    // Sessions
    SessionRevoke,
    SessionRevokeAll,
    // Account deletion (GDPR)
    AccountDeleteRequest,
    AccountDeleteCancel,
    AccountDeleteExecute,
    // Security alerts
    RateLimitExceeded,
    SuspiciousActivity,
}

impl SecurityEventType {
    /// Convert to database enum string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RegisterSuccess => "register_success",
            Self::RegisterFailure => "register_failure",
            Self::LoginSuccess => "login_success",
            Self::LoginFailure => "login_failure",
            Self::LoginLocked => "login_locked",
            Self::Logout => "logout",
            Self::PasswordResetRequest => "password_reset_request",
            Self::PasswordResetComplete => "password_reset_complete",
            Self::PasswordChange => "password_change",
            Self::EmailChange => "email_change",
            Self::EmailConfirmed => "email_confirmed",
            Self::TotpEnable => "totp_enable",
            Self::TotpDisable => "totp_disable",
            Self::RecoveryCodeUsed => "recovery_code_used",
            Self::SessionRevoke => "session_revoke",
            Self::SessionRevokeAll => "session_revoke_all",
            Self::AccountDeleteRequest => "account_delete_request",
            Self::AccountDeleteCancel => "account_delete_cancel",
            Self::AccountDeleteExecute => "account_delete_execute",
            Self::RateLimitExceeded => "rate_limit_exceeded",
            Self::SuspiciousActivity => "suspicious_activity",
        }
    }

    /// Get the tracing log level for this event type
    pub fn log_level(&self) -> tracing::Level {
        match self {
            Self::RegisterSuccess
            | Self::LoginSuccess
            | Self::Logout
            | Self::PasswordResetRequest
            | Self::PasswordResetComplete
            | Self::PasswordChange
            | Self::EmailChange
            | Self::EmailConfirmed
            | Self::TotpEnable
            | Self::SessionRevoke
            | Self::AccountDeleteCancel => tracing::Level::INFO,

            Self::RegisterFailure
            | Self::LoginFailure
            | Self::LoginLocked
            | Self::TotpDisable
            | Self::RecoveryCodeUsed
            | Self::SessionRevokeAll
            | Self::AccountDeleteRequest
            | Self::AccountDeleteExecute
            | Self::RateLimitExceeded => tracing::Level::WARN,

            Self::SuspiciousActivity => tracing::Level::ERROR,
        }
    }
}

/// Builder for creating security events with metadata
pub struct SecurityEventBuilder {
    user_id: Option<Uuid>,
    event_type: SecurityEventType,
    ip_address: Option<IpAddr>,
    user_agent: Option<String>,
    metadata: serde_json::Map<String, serde_json::Value>,
}

impl SecurityEventBuilder {
    /// Create a new event builder
    pub fn new(event_type: SecurityEventType) -> Self {
        Self {
            user_id: None,
            event_type,
            ip_address: None,
            user_agent: None,
            metadata: serde_json::Map::new(),
        }
    }

    /// Set the user ID
    pub fn user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set optional user ID
    pub fn maybe_user(mut self, user_id: Option<Uuid>) -> Self {
        self.user_id = user_id;
        self
    }

    /// Set the IP address
    pub fn ip(mut self, ip: IpAddr) -> Self {
        self.ip_address = Some(ip);
        self
    }

    /// Set optional IP address
    pub fn maybe_ip(mut self, ip: Option<IpAddr>) -> Self {
        self.ip_address = ip;
        self
    }

    /// Set the user agent
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    /// Set optional user agent
    pub fn maybe_user_agent(mut self, ua: Option<String>) -> Self {
        self.user_agent = ua;
        self
    }

    /// Add metadata (do not include PII - use email domain, not full email)
    pub fn with_metadata(mut self, key: &str, value: impl Into<serde_json::Value>) -> Self {
        self.metadata.insert(key.to_string(), value.into());
        self
    }

    /// Add email domain to metadata (for GDPR compliance - no full email)
    pub fn with_email_domain(self, email: &str) -> Self {
        let domain = email.split('@').next_back().unwrap_or("unknown");
        self.with_metadata("email_domain", domain)
    }
}

/// Security logging service
#[derive(Clone)]
pub struct SecurityLogService {
    pool: PgPool,
}

impl SecurityLogService {
    /// Create a new security log service
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Log a security event
    pub async fn log(&self, event: SecurityEventBuilder) -> Result<Uuid, sqlx::Error> {
        let metadata = if event.metadata.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(event.metadata.clone()))
        };

        let ip_str = event.ip_address.map(|ip| ip.to_string());

        // Log to tracing as well
        let level = event.event_type.log_level();
        match level {
            tracing::Level::INFO => {
                tracing::info!(
                    event_type = event.event_type.as_str(),
                    user_id = ?event.user_id,
                    ip = ?event.ip_address,
                    "Security event"
                );
            }
            tracing::Level::WARN => {
                tracing::warn!(
                    event_type = event.event_type.as_str(),
                    user_id = ?event.user_id,
                    ip = ?event.ip_address,
                    "Security event"
                );
            }
            tracing::Level::ERROR => {
                tracing::error!(
                    event_type = event.event_type.as_str(),
                    user_id = ?event.user_id,
                    ip = ?event.ip_address,
                    "Security event"
                );
            }
            _ => {}
        }

        // Insert into database
        // Use raw query to handle the enum type
        let result = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO security_events (user_id, event_type, ip_address, user_agent, metadata)
            VALUES ($1, $2::security_event_type, $3::inet, $4, $5)
            RETURNING id
            "#,
        )
        .bind(event.user_id)
        .bind(event.event_type.as_str())
        .bind(ip_str)
        .bind(&event.user_agent)
        .bind(&metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// Log a registration success event
    pub async fn register_success(
        &self,
        user_id: Uuid,
        email: &str,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::RegisterSuccess)
                .user(user_id)
                .maybe_ip(ip)
                .with_email_domain(email),
        )
        .await
    }

    /// Log a registration failure event
    pub async fn register_failure(
        &self,
        email: &str,
        reason: &str,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::RegisterFailure)
                .maybe_ip(ip)
                .with_email_domain(email)
                .with_metadata("reason", reason),
        )
        .await
    }

    /// Log a login success event
    pub async fn login_success(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
        user_agent: Option<String>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::LoginSuccess)
                .user(user_id)
                .maybe_ip(ip)
                .maybe_user_agent(user_agent),
        )
        .await
    }

    /// Log a login failure event
    pub async fn login_failure(
        &self,
        email: &str,
        reason: &str,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::LoginFailure)
                .maybe_ip(ip)
                .with_email_domain(email)
                .with_metadata("reason", reason),
        )
        .await
    }

    /// Log an account locked event
    pub async fn login_locked(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
        locked_until: &str,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::LoginLocked)
                .user(user_id)
                .maybe_ip(ip)
                .with_metadata("locked_until", locked_until),
        )
        .await
    }

    /// Log a logout event
    pub async fn logout(&self, user_id: Uuid, session_id: Uuid) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::Logout)
                .user(user_id)
                .with_metadata("session_id", session_id.to_string()),
        )
        .await
    }

    /// Log a password reset request
    pub async fn password_reset_request(
        &self,
        email: &str,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::PasswordResetRequest)
                .maybe_ip(ip)
                .with_email_domain(email),
        )
        .await
    }

    /// Log a password reset completion
    pub async fn password_reset_complete(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::PasswordResetComplete)
                .user(user_id)
                .maybe_ip(ip),
        )
        .await
    }

    /// Log a password change
    pub async fn password_change(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::PasswordChange)
                .user(user_id)
                .maybe_ip(ip),
        )
        .await
    }

    /// Log an email change
    pub async fn email_change(
        &self,
        user_id: Uuid,
        old_email: &str,
        new_email: &str,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        let old_domain = old_email.split('@').next_back().unwrap_or("unknown");
        let new_domain = new_email.split('@').next_back().unwrap_or("unknown");

        self.log(
            SecurityEventBuilder::new(SecurityEventType::EmailChange)
                .user(user_id)
                .maybe_ip(ip)
                .with_metadata("old_domain", old_domain)
                .with_metadata("new_domain", new_domain),
        )
        .await
    }

    /// Log email confirmation
    pub async fn email_confirmed(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::EmailConfirmed)
                .user(user_id)
                .maybe_ip(ip),
        )
        .await
    }

    /// Log 2FA enable
    pub async fn totp_enable(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::TotpEnable)
                .user(user_id)
                .maybe_ip(ip),
        )
        .await
    }

    /// Log 2FA disable
    pub async fn totp_disable(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::TotpDisable)
                .user(user_id)
                .maybe_ip(ip),
        )
        .await
    }

    /// Log recovery code usage
    pub async fn recovery_code_used(
        &self,
        user_id: Uuid,
        codes_remaining: u8,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::RecoveryCodeUsed)
                .user(user_id)
                .maybe_ip(ip)
                .with_metadata("codes_remaining", codes_remaining),
        )
        .await
    }

    /// Log session revocation
    pub async fn session_revoke(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::SessionRevoke)
                .user(user_id)
                .maybe_ip(ip)
                .with_metadata("session_id", session_id.to_string()),
        )
        .await
    }

    /// Log all sessions revoked
    pub async fn session_revoke_all(
        &self,
        user_id: Uuid,
        count: u32,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::SessionRevokeAll)
                .user(user_id)
                .maybe_ip(ip)
                .with_metadata("sessions_revoked", count),
        )
        .await
    }

    /// Log account deletion request
    pub async fn account_delete_request(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::AccountDeleteRequest)
                .user(user_id)
                .maybe_ip(ip),
        )
        .await
    }

    /// Log account deletion cancellation
    pub async fn account_delete_cancel(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::AccountDeleteCancel)
                .user(user_id)
                .maybe_ip(ip),
        )
        .await
    }

    /// Log account deletion execution
    pub async fn account_delete_execute(
        &self,
        user_id: Uuid,
        recipes_orphaned: u32,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::AccountDeleteExecute)
                .user(user_id)
                .with_metadata("recipes_orphaned", recipes_orphaned),
        )
        .await
    }

    /// Log rate limit exceeded
    pub async fn rate_limit_exceeded(
        &self,
        ip: Option<IpAddr>,
        endpoint: &str,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::RateLimitExceeded)
                .maybe_ip(ip)
                .with_metadata("endpoint", endpoint),
        )
        .await
    }

    /// Log suspicious activity
    pub async fn suspicious_activity(
        &self,
        user_id: Option<Uuid>,
        ip: Option<IpAddr>,
        reason: &str,
    ) -> Result<Uuid, sqlx::Error> {
        self.log(
            SecurityEventBuilder::new(SecurityEventType::SuspiciousActivity)
                .maybe_user(user_id)
                .maybe_ip(ip)
                .with_metadata("reason", reason),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_event_type_as_str() {
        assert_eq!(SecurityEventType::LoginSuccess.as_str(), "login_success");
        assert_eq!(
            SecurityEventType::RegisterFailure.as_str(),
            "register_failure"
        );
    }

    #[test]
    fn test_event_builder() {
        let event = SecurityEventBuilder::new(SecurityEventType::LoginSuccess)
            .user(Uuid::new_v4())
            .with_email_domain("user@example.com")
            .with_metadata("extra", "value");

        assert!(event.user_id.is_some());
        assert_eq!(
            event.metadata.get("email_domain"),
            Some(&json!("example.com"))
        );
    }
}
