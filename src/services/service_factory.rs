//! Service factory module
//!
//! Provides centralized service construction to avoid code duplication
//! across handlers and API modules.

use sqlx::PgPool;

use crate::core::config::SmtpConfig;
use crate::core::error::AppError;
use crate::services::{AuthService, EmailService, PasswordService, SessionService, TotpService};

/// Default session expiry in days
pub const SESSION_EXPIRY_DAYS: u32 = 30;

/// Service factory for creating commonly used service instances
pub struct ServiceFactory;

impl ServiceFactory {
    /// Create an AuthService instance
    ///
    /// Uses environment variables for configuration:
    /// - `BASE_URL`: Base URL for links (default: http://localhost:3000)
    /// - `APP_ENV`: Environment setting (production enables strict SMTP)
    /// - `HIBP_ENABLED`: Enable Have I Been Pwned password checking (default: true)
    pub fn create_auth_service(db: PgPool) -> AuthService {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let is_production = std::env::var("APP_ENV")
            .map(|v| v == "production")
            .unwrap_or(false);

        let password_service = PasswordService::new(
            std::env::var("HIBP_ENABLED")
                .map(|v| v == "true")
                .unwrap_or(true),
        );

        let smtp_config = SmtpConfig::from_env(is_production);
        let email_service = EmailService::new(smtp_config, base_url.clone());

        AuthService::new(
            db,
            password_service,
            email_service,
            base_url,
            SESSION_EXPIRY_DAYS,
        )
    }

    /// Create an EmailService instance
    pub fn create_email_service() -> EmailService {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let is_production = std::env::var("APP_ENV")
            .map(|v| v == "production")
            .unwrap_or(false);

        let smtp_config = SmtpConfig::from_env(is_production);
        EmailService::new(smtp_config, base_url)
    }

    /// Create a SessionService instance using the default session expiry
    pub fn create_session_service(db: PgPool) -> SessionService {
        SessionService::new(db, SESSION_EXPIRY_DAYS)
    }

    /// Create a TotpService instance
    ///
    /// Returns an error if TOTP_ENCRYPTION_KEY is not properly configured.
    pub fn create_totp_service(db: PgPool) -> Result<TotpService, AppError> {
        TotpService::from_env(db)
            .map_err(|e| AppError::Internal(format!("TOTP setup error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_expiry_days_constant() {
        assert_eq!(SESSION_EXPIRY_DAYS, 30);
    }
}
