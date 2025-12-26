//! Email confirmation token model
//!
//! Handles email verification for new registrations and email changes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Email confirmation token record
#[derive(Debug, Clone, FromRow)]
pub struct EmailConfirmationToken {
    pub id: Uuid,
    /// User ID - present for email change, null for initial registration
    pub user_id: Option<Uuid>,
    /// Email address being confirmed
    pub email: String,
    /// SHA-256 hash of the token (never store plaintext)
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Create a new email confirmation token
#[derive(Debug)]
pub struct CreateEmailConfirmationToken {
    pub user_id: Option<Uuid>,
    pub email: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
}

/// Email confirmation request DTO
#[derive(Debug, Clone, Deserialize)]
pub struct ConfirmEmailRequest {
    pub token: String,
}

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

/// Token info for display (without sensitive hash)
#[derive(Debug, Clone, Serialize)]
pub struct EmailConfirmationInfo {
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_expired: bool,
}

impl EmailConfirmationToken {
    /// Check if the token has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    /// Convert to info struct (for API responses)
    pub fn to_info(&self) -> EmailConfirmationInfo {
        EmailConfirmationInfo {
            email: self.email.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
            is_expired: self.is_expired(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_token_expiry() {
        let expired_token = EmailConfirmationToken {
            id: Uuid::new_v4(),
            user_id: None,
            email: "test@example.com".to_string(),
            token_hash: "hash".to_string(),
            created_at: Utc::now() - Duration::hours(25),
            expires_at: Utc::now() - Duration::hours(1),
        };
        assert!(expired_token.is_expired());

        let valid_token = EmailConfirmationToken {
            id: Uuid::new_v4(),
            user_id: None,
            email: "test@example.com".to_string(),
            token_hash: "hash".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(24),
        };
        assert!(!valid_token.is_expired());
    }

    #[test]
    fn test_to_info() {
        let token = EmailConfirmationToken {
            id: Uuid::new_v4(),
            user_id: Some(Uuid::new_v4()),
            email: "test@example.com".to_string(),
            token_hash: "secret_hash".to_string(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::hours(24),
        };

        let info = token.to_info();
        assert_eq!(info.email, "test@example.com");
        assert!(!info.is_expired);
    }

    #[test]
    fn test_resend_confirmation_validation() {
        use validator::Validate;

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
