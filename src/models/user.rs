use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Reserved usernames that cannot be registered
pub const RESERVED_USERNAMES: &[&str] = &[
    "admin",
    "root",
    "system",
    "support",
    "help",
    "oppskrift",
    "api",
    "auth",
    "login",
    "logout",
    "register",
    "settings",
    "account",
    "profile",
    "user",
    "users",
    "mod",
    "moderator",
];

/// User measurement preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "measurement_pref", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MeasurementPref {
    #[default]
    Metric,
    Imperial,
}

/// What happens to user's content on account deletion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "deletion_content_choice", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DeletionContentChoice {
    /// Keep recipes/books with "Deleted User" attribution
    Anonymize,
    /// Remove all user's content (recipes, books, comments)
    DeleteAll,
}

/// User entity - represents both local and federated users
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    /// Email address (None for federated-only users)
    pub email: Option<String>,
    pub email_verified: bool,
    #[serde(skip_serializing)] // Never expose password hash
    /// Password hash (None for federated-only users)
    pub password_hash: Option<String>,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub measurement_pref: MeasurementPref,
    // Selected by sqlx but accessed via direct queries, not through struct
    #[serde(skip_serializing)]
    #[sqlx(rename = "totp_secret_encrypted")]
    pub _totp_secret_encrypted: Option<Vec<u8>>,
    pub totp_enabled: bool,
    #[serde(skip_serializing)]
    #[sqlx(rename = "failed_login_attempts")]
    pub _failed_login_attempts: i32,
    #[serde(skip_serializing)]
    pub locked_until: Option<DateTime<Utc>>,
    pub deletion_requested_at: Option<DateTime<Utc>>,
    pub deletion_content_choice: Option<DeletionContentChoice>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ap_id: String,
    /// Whether user participates in ActivityPub federation
    pub federation_enabled: bool,
}

/// Create a new user (for registration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub measurement_pref: Option<MeasurementPref>,
    pub ap_id: String,
}

/// Update user profile
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateUser {
    #[validate(length(min = 1, max = 100, message = "Display name must be 1-100 characters"))]
    pub display_name: Option<String>,
    #[validate(length(max = 500, message = "Bio must be at most 500 characters"))]
    pub bio: Option<String>,
    #[validate(url(message = "Avatar URL must be a valid URL"))]
    pub avatar_url: Option<String>,
    pub measurement_pref: Option<MeasurementPref>,
}

/// Public user profile (safe to expose)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub ap_id: String,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            display_name: user.display_name,
            bio: user.bio,
            avatar_url: user.avatar_url,
            created_at: user.created_at,
            ap_id: user.ap_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measurement_pref_default() {
        assert_eq!(MeasurementPref::default(), MeasurementPref::Metric);
    }

    #[test]
    fn test_user_profile_from_user() {
        let user = User {
            id: Uuid::new_v4(),
            username: "chef".to_string(),
            email: Some("chef@example.com".to_string()),
            email_verified: true,
            password_hash: Some("hashed".to_string()),
            display_name: "Chef Marie".to_string(),
            bio: Some("I love cooking".to_string()),
            avatar_url: None,
            measurement_pref: MeasurementPref::Metric,
            _totp_secret_encrypted: None,
            totp_enabled: false,
            _failed_login_attempts: 0,
            locked_until: None,
            deletion_requested_at: None,
            deletion_content_choice: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/users/chef".to_string(),
            federation_enabled: true,
        };

        let profile: UserProfile = user.clone().into();
        assert_eq!(profile.username, user.username);
        assert_eq!(profile.display_name, user.display_name);
    }

    #[test]
    fn test_reserved_usernames() {
        assert!(RESERVED_USERNAMES.contains(&"admin"));
        assert!(RESERVED_USERNAMES.contains(&"api"));
        assert!(!RESERVED_USERNAMES.contains(&"chef"));
    }
}
