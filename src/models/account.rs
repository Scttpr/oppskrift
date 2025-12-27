//! Account management DTOs
//!
//! Structs for password change, email change, and account settings.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request to change password
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    /// Current password for verification
    #[validate(length(min = 1, message = "Current password is required"))]
    pub current_password: String,
    /// New password (must meet password requirements)
    #[validate(length(min = 10, message = "Password must be at least 10 characters"))]
    pub new_password: String,
}

/// Response after password change
#[derive(Debug, Serialize)]
pub struct ChangePasswordResponse {
    pub message: String,
    /// Number of sessions invalidated
    pub sessions_revoked: u32,
}

impl ChangePasswordResponse {
    pub fn success(sessions_revoked: u32) -> Self {
        Self {
            message: "Password changed successfully. Other sessions have been logged out."
                .to_string(),
            sessions_revoked,
        }
    }
}

/// Request to change email address
#[derive(Debug, Deserialize, Validate)]
pub struct ChangeEmailRequest {
    /// New email address
    #[validate(email(message = "Invalid email format"))]
    pub new_email: String,
    /// Current password for verification
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// Response after email change request
#[derive(Debug, Serialize)]
pub struct ChangeEmailResponse {
    pub message: String,
}

impl ChangeEmailResponse {
    pub fn success() -> Self {
        Self {
            message: "A confirmation link has been sent to your new email address.".to_string(),
        }
    }
}

/// Request to delete account
#[derive(Debug, Deserialize, Validate)]
pub struct DeleteAccountRequest {
    /// Password confirmation
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// Response after deletion request
#[derive(Debug, Serialize)]
pub struct DeletionScheduledResponse {
    pub message: String,
    /// Date when deletion will be executed
    pub scheduled_for: chrono::DateTime<chrono::Utc>,
    /// Number of days in grace period
    pub grace_period_days: u32,
}

impl DeletionScheduledResponse {
    pub fn new(scheduled_for: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            message: "Account deletion scheduled. You can cancel within the grace period."
                .to_string(),
            scheduled_for,
            grace_period_days: 7,
        }
    }
}

/// Response after cancelling deletion
#[derive(Debug, Serialize)]
pub struct CancelDeletionResponse {
    pub message: String,
}

impl CancelDeletionResponse {
    pub fn success() -> Self {
        Self {
            message: "Account deletion cancelled. Your account has been restored.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_change_password_request_validation() {
        let valid = ChangePasswordRequest {
            current_password: "currentpass".to_string(),
            new_password: "NewSecurePass123".to_string(),
        };
        assert!(valid.validate().is_ok());

        let short_password = ChangePasswordRequest {
            current_password: "currentpass".to_string(),
            new_password: "short".to_string(),
        };
        assert!(short_password.validate().is_err());

        let empty_current = ChangePasswordRequest {
            current_password: "".to_string(),
            new_password: "NewSecurePass123".to_string(),
        };
        assert!(empty_current.validate().is_err());
    }

    #[test]
    fn test_change_email_request_validation() {
        let valid = ChangeEmailRequest {
            new_email: "new@example.com".to_string(),
            password: "mypassword".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid_email = ChangeEmailRequest {
            new_email: "not-an-email".to_string(),
            password: "mypassword".to_string(),
        };
        assert!(invalid_email.validate().is_err());
    }

    #[test]
    fn test_deletion_response() {
        let scheduled = chrono::Utc::now() + chrono::Duration::days(7);
        let response = DeletionScheduledResponse::new(scheduled);

        assert_eq!(response.grace_period_days, 7);
        assert!(response.message.contains("grace period"));
    }
}
