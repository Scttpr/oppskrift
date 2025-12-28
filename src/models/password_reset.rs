//! Password reset DTOs

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Request password reset
#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}

/// Reset password with token
#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[validate(length(min = 10, message = "Password must be at least 10 characters"))]
    pub new_password: String,
}

/// Response for password reset request (always same message for security)
#[derive(Debug, Serialize)]
pub struct ForgotPasswordResponse {
    pub message: String,
}

impl ForgotPasswordResponse {
    pub fn success() -> Self {
        Self {
            message: "If an account exists with this email, a password reset link has been sent."
                .to_string(),
        }
    }
}

/// Response for password reset completion
#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    pub message: String,
}

impl ResetPasswordResponse {
    pub fn success() -> Self {
        Self {
            message: "Password reset successfully. You can now log in with your new password."
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_forgot_password_request_validation() {
        let valid = ForgotPasswordRequest {
            email: "test@example.com".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid = ForgotPasswordRequest {
            email: "not-an-email".to_string(),
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_reset_password_request_validation() {
        let valid = ResetPasswordRequest {
            token: "abc123".to_string(),
            new_password: "SecureNewPass123".to_string(),
        };
        assert!(valid.validate().is_ok());

        let short_password = ResetPasswordRequest {
            token: "abc123".to_string(),
            new_password: "short".to_string(),
        };
        assert!(short_password.validate().is_err());
    }

    #[test]
    fn test_response_messages() {
        let forgot_response = ForgotPasswordResponse::success();
        assert!(forgot_response.message.contains("If an account exists"));

        let reset_response = ResetPasswordResponse::success();
        assert!(reset_response.message.contains("successfully"));
    }
}
