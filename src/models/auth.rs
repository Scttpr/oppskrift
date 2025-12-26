//! Authentication DTOs for registration, login, and 2FA
//!
//! These structs handle API request/response serialization with validation.

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::UserProfile;

lazy_static! {
    /// Username must be lowercase alphanumeric with underscores
    static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-z0-9_]+$").unwrap();
}

/// Registration request
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(
        length(min = 3, max = 30, message = "Username must be 3-30 characters"),
        regex(path = *USERNAME_REGEX, message = "Username can only contain a-z, 0-9, and _")
    )]
    pub username: String,

    #[validate(length(min = 10, message = "Password must be at least 10 characters"))]
    pub password: String,

    #[validate(length(min = 1, max = 100, message = "Display name must be 1-100 characters"))]
    pub display_name: Option<String>,
}

/// Registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub message: String,
    pub user_id: Uuid,
}

/// Login request
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    pub password: String,
    /// TOTP code - required if 2FA is enabled on account
    pub totp_code: Option<String>,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserProfile,
    pub expires_at: DateTime<Utc>,
}

/// 2FA required response (intermediate login state)
#[derive(Debug, Serialize)]
pub struct TwoFactorRequired {
    pub message: String,
    /// Partial session token, only valid for 2FA completion
    pub partial_token: String,
}

/// Logout response
#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub message: String,
}

/// Generic auth error response
#[derive(Debug, Serialize)]
pub struct AuthError {
    pub error: String,
    pub code: String,
}

impl AuthError {
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
        }
    }

    pub fn invalid_credentials() -> Self {
        Self::new("Invalid email or password", "INVALID_CREDENTIALS")
    }

    pub fn account_locked(until: DateTime<Utc>) -> Self {
        Self::new(
            format!(
                "Account locked until {}",
                until.format("%Y-%m-%d %H:%M UTC")
            ),
            "ACCOUNT_LOCKED",
        )
    }

    pub fn email_not_verified() -> Self {
        Self::new(
            "Please verify your email before logging in",
            "EMAIL_NOT_VERIFIED",
        )
    }

    pub fn requires_2fa() -> Self {
        Self::new("Two-factor authentication required", "REQUIRES_2FA")
    }

    pub fn invalid_2fa_code() -> Self {
        Self::new("Invalid two-factor authentication code", "INVALID_2FA_CODE")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_register_request_validation() {
        let valid = RegisterRequest {
            email: "test@example.com".to_string(),
            username: "valid_user".to_string(),
            password: "SecurePass123".to_string(),
            display_name: Some("Test User".to_string()),
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_register_request_invalid_email() {
        let invalid = RegisterRequest {
            email: "not-an-email".to_string(),
            username: "valid_user".to_string(),
            password: "SecurePass123".to_string(),
            display_name: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_register_request_invalid_username() {
        let invalid = RegisterRequest {
            email: "test@example.com".to_string(),
            username: "Invalid-User".to_string(), // Contains uppercase and dash
            password: "SecurePass123".to_string(),
            display_name: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_register_request_short_password() {
        let invalid = RegisterRequest {
            email: "test@example.com".to_string(),
            username: "valid_user".to_string(),
            password: "short".to_string(), // Less than 10 chars
            display_name: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_auth_error_messages() {
        let locked = AuthError::account_locked(Utc::now());
        assert!(locked.error.contains("locked"));
        assert_eq!(locked.code, "ACCOUNT_LOCKED");
    }
}
