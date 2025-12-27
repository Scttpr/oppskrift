//! Email confirmation models

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Resend confirmation email request
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ResendConfirmationRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}

/// Email confirmation response
#[derive(Debug, Serialize)]
pub struct EmailConfirmationResponse {
    pub message: String,
    pub verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resend_confirmation_validation() {
        let valid = ResendConfirmationRequest {
            email: "valid@example.com".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid = ResendConfirmationRequest {
            email: "not-an-email".to_string(),
        };
        assert!(invalid.validate().is_err());
    }
}
