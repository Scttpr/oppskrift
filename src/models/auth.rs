//! Authentication DTOs for registration, login, and 2FA
//!
//! These structs handle API request/response serialization with validation.

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use super::UserProfile;

lazy_static! {
    /// Username must be lowercase alphanumeric with underscores
    static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-z0-9_]+$").unwrap();
}

/// Registration request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email(message = "Format d'e-mail invalide"))]
    pub email: String,

    #[validate(
        length(min = 3, max = 30, message = "Le nom d'utilisateur doit comporter de 3 à 30 caractères"),
        regex(path = *USERNAME_REGEX, message = "Le nom d'utilisateur ne peut contenir que a-z, 0-9 et _")
    )]
    pub username: String,

    #[validate(length(
        min = 10,
        message = "Le mot de passe doit comporter au moins 10 caractères"
    ))]
    pub password: String,

    #[validate(length(
        min = 1,
        max = 100,
        message = "Le nom affiché doit comporter de 1 à 100 caractères"
    ))]
    pub display_name: Option<String>,
}

/// Registration response
#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResponse {
    pub message: String,
    pub user_id: Uuid,
}

/// Login request
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email(message = "Format d'e-mail invalide"))]
    pub email: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub user: UserProfile,
    pub expires_at: DateTime<Utc>,
}

/// Logout response
#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutResponse {
    pub message: String,
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
}
