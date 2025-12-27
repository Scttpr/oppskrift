//! Two-factor authentication DTOs
//!
//! Structs for TOTP-based 2FA setup, verification, and management.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Response for 2FA setup - contains QR code and secret
#[derive(Debug, Serialize)]
pub struct TwoFactorSetupResponse {
    /// Base64-encoded QR code PNG image
    pub qr_code: String,
    /// TOTP secret for manual entry (base32 encoded)
    pub secret: String,
    /// otpauth:// URI for authenticator apps
    pub otpauth_uri: String,
}

/// Request to enable 2FA - requires verification of TOTP code
#[derive(Debug, Deserialize, Validate)]
pub struct EnableTwoFactorRequest {
    /// TOTP code from authenticator app (6 digits)
    #[validate(length(equal = 6, message = "TOTP code must be 6 digits"))]
    #[validate(custom(function = "validate_totp_code"))]
    pub totp_code: String,
}

/// Request to disable 2FA - requires password and TOTP/recovery code
#[derive(Debug, Deserialize, Validate)]
pub struct DisableTwoFactorRequest {
    /// Current password for verification
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
    /// TOTP code or recovery code
    #[validate(length(min = 1, message = "Code is required"))]
    pub code: String,
}

/// Response after 2FA is enabled - includes recovery codes
#[derive(Debug, Serialize)]
pub struct TwoFactorEnabledResponse {
    pub message: String,
    /// One-time recovery codes (shown only once)
    pub recovery_codes: Vec<String>,
}

/// Request to complete 2FA login verification
#[derive(Debug, Deserialize, Validate)]
pub struct Complete2FALoginRequest {
    /// Partial token from initial login response
    #[validate(length(equal = 64, message = "Invalid partial token"))]
    pub partial_token: String,
    /// TOTP code from authenticator app (6 digits)
    #[validate(length(equal = 6, message = "TOTP code must be 6 digits"))]
    #[validate(custom(function = "validate_totp_code"))]
    pub totp_code: String,
}

/// 2FA status response
#[derive(Debug, Serialize)]
pub struct TwoFactorStatusResponse {
    pub enabled: bool,
    /// Number of remaining recovery codes
    pub recovery_codes_remaining: u32,
}

/// Validate that TOTP code contains only digits
fn validate_totp_code(code: &str) -> Result<(), validator::ValidationError> {
    if code.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("totp_code_format"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_enable_2fa_request_validation() {
        let valid = EnableTwoFactorRequest {
            totp_code: "123456".to_string(),
        };
        assert!(valid.validate().is_ok());

        let invalid_length = EnableTwoFactorRequest {
            totp_code: "12345".to_string(),
        };
        assert!(invalid_length.validate().is_err());

        let invalid_chars = EnableTwoFactorRequest {
            totp_code: "12345a".to_string(),
        };
        assert!(invalid_chars.validate().is_err());
    }

    #[test]
    fn test_disable_2fa_request_validation() {
        let valid = DisableTwoFactorRequest {
            password: "mypassword".to_string(),
            code: "123456".to_string(),
        };
        assert!(valid.validate().is_ok());

        let missing_password = DisableTwoFactorRequest {
            password: "".to_string(),
            code: "123456".to_string(),
        };
        assert!(missing_password.validate().is_err());
    }
}
