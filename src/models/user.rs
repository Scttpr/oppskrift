use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// User measurement preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "measurement_pref", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MeasurementPref {
    #[default]
    Metric,
    Imperial,
}

/// User entity - represents both local and federated users
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub measurement_pref: MeasurementPref,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ap_id: String,
    /// Whether user participates in ActivityPub federation
    pub federation_enabled: bool,
}

/// Create a new user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUser {
    pub username: String,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
            display_name: "Chef Marie".to_string(),
            bio: Some("I love cooking".to_string()),
            avatar_url: None,
            measurement_pref: MeasurementPref::Metric,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/users/chef".to_string(),
            federation_enabled: true,
        };

        let profile: UserProfile = user.clone().into();
        assert_eq!(profile.username, user.username);
        assert_eq!(profile.display_name, user.display_name);
    }
}
