use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use uuid::Uuid;

/// Type of activity performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "activity_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ActivityType {
    Create,
    Share,
    Follow,
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityType::Create => write!(f, "create"),
            ActivityType::Share => write!(f, "share"),
            ActivityType::Follow => write!(f, "follow"),
        }
    }
}

/// Type of target entity for the activity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[sqlx(type_name = "target_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TargetType {
    Recipe,
    Book,
    User,
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Recipe => write!(f, "recipe"),
            TargetType::Book => write!(f, "book"),
            TargetType::User => write!(f, "user"),
        }
    }
}

/// An activity in the feed (create, share, follow)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Activity {
    pub id: Uuid,
    pub actor_id: Uuid,
    pub activity_type: ActivityType,
    pub target_type: TargetType,
    pub target_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub ap_id: String,
}

/// Input for creating an activity
#[derive(Debug, Clone)]
pub struct CreateActivity {
    pub actor_id: Uuid,
    pub activity_type: ActivityType,
    pub target_type: TargetType,
    pub target_id: Uuid,
}

/// Activity with resolved actor information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityWithActor {
    pub id: Uuid,
    pub actor_id: Uuid,
    pub actor_username: String,
    pub actor_display_name: String,
    pub actor_avatar_url: Option<String>,
    pub activity_type: ActivityType,
    pub target_type: TargetType,
    pub target_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_type_display() {
        assert_eq!(ActivityType::Create.to_string(), "create");
        assert_eq!(ActivityType::Share.to_string(), "share");
        assert_eq!(ActivityType::Follow.to_string(), "follow");
    }

    #[test]
    fn test_target_type_display() {
        assert_eq!(TargetType::Recipe.to_string(), "recipe");
        assert_eq!(TargetType::Book.to_string(), "book");
        assert_eq!(TargetType::User.to_string(), "user");
    }

    #[test]
    fn test_activity_serialization() {
        let activity = Activity {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/activities/123".to_string(),
        };

        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("\"activity_type\":\"create\""));
        assert!(json.contains("\"target_type\":\"recipe\""));
    }
}
