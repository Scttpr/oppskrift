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

    // ==========================================================================
    // ActivityType Tests (T049)
    // ==========================================================================

    #[test]
    fn test_activity_type_display() {
        assert_eq!(ActivityType::Create.to_string(), "create");
        assert_eq!(ActivityType::Share.to_string(), "share");
        assert_eq!(ActivityType::Follow.to_string(), "follow");
    }

    #[test]
    fn test_activity_type_serialization_roundtrip() {
        for activity_type in [
            ActivityType::Create,
            ActivityType::Share,
            ActivityType::Follow,
        ] {
            let json = serde_json::to_string(&activity_type).unwrap();
            let deserialized: ActivityType = serde_json::from_str(&json).unwrap();
            assert_eq!(activity_type, deserialized);
        }
    }

    #[test]
    fn test_activity_type_json_format() {
        assert_eq!(
            serde_json::to_string(&ActivityType::Create).unwrap(),
            "\"create\""
        );
        assert_eq!(
            serde_json::to_string(&ActivityType::Share).unwrap(),
            "\"share\""
        );
        assert_eq!(
            serde_json::to_string(&ActivityType::Follow).unwrap(),
            "\"follow\""
        );
    }

    #[test]
    fn test_activity_type_deserialization() {
        let create: ActivityType = serde_json::from_str("\"create\"").unwrap();
        assert_eq!(create, ActivityType::Create);

        let share: ActivityType = serde_json::from_str("\"share\"").unwrap();
        assert_eq!(share, ActivityType::Share);

        let follow: ActivityType = serde_json::from_str("\"follow\"").unwrap();
        assert_eq!(follow, ActivityType::Follow);
    }

    #[test]
    fn test_activity_type_copy() {
        let original = ActivityType::Create;
        let copied = original;
        assert_eq!(original, copied);
    }

    // ==========================================================================
    // TargetType Tests (T049)
    // ==========================================================================

    #[test]
    fn test_target_type_display() {
        assert_eq!(TargetType::Recipe.to_string(), "recipe");
        assert_eq!(TargetType::Book.to_string(), "book");
        assert_eq!(TargetType::User.to_string(), "user");
    }

    #[test]
    fn test_target_type_serialization_roundtrip() {
        for target_type in [TargetType::Recipe, TargetType::Book, TargetType::User] {
            let json = serde_json::to_string(&target_type).unwrap();
            let deserialized: TargetType = serde_json::from_str(&json).unwrap();
            assert_eq!(target_type, deserialized);
        }
    }

    #[test]
    fn test_target_type_json_format() {
        assert_eq!(
            serde_json::to_string(&TargetType::Recipe).unwrap(),
            "\"recipe\""
        );
        assert_eq!(
            serde_json::to_string(&TargetType::Book).unwrap(),
            "\"book\""
        );
        assert_eq!(
            serde_json::to_string(&TargetType::User).unwrap(),
            "\"user\""
        );
    }

    #[test]
    fn test_target_type_deserialization() {
        let recipe: TargetType = serde_json::from_str("\"recipe\"").unwrap();
        assert_eq!(recipe, TargetType::Recipe);

        let book: TargetType = serde_json::from_str("\"book\"").unwrap();
        assert_eq!(book, TargetType::Book);

        let user: TargetType = serde_json::from_str("\"user\"").unwrap();
        assert_eq!(user, TargetType::User);
    }

    #[test]
    fn test_target_type_copy() {
        let original = TargetType::Recipe;
        let copied = original;
        assert_eq!(original, copied);
    }

    // ==========================================================================
    // Activity Serialization Tests (T049)
    // ==========================================================================

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

    #[test]
    fn test_activity_serialization_roundtrip() {
        let activity = Activity {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            activity_type: ActivityType::Share,
            target_type: TargetType::Book,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/activities/456".to_string(),
        };

        let json = serde_json::to_string(&activity).unwrap();
        let deserialized: Activity = serde_json::from_str(&json).unwrap();

        assert_eq!(activity.id, deserialized.id);
        assert_eq!(activity.actor_id, deserialized.actor_id);
        assert_eq!(activity.activity_type, deserialized.activity_type);
        assert_eq!(activity.target_type, deserialized.target_type);
        assert_eq!(activity.target_id, deserialized.target_id);
        assert_eq!(activity.ap_id, deserialized.ap_id);
    }

    #[test]
    fn test_activity_follow_user() {
        let activity = Activity {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            activity_type: ActivityType::Follow,
            target_type: TargetType::User,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/activities/follow".to_string(),
        };

        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("\"activity_type\":\"follow\""));
        assert!(json.contains("\"target_type\":\"user\""));
    }

    // ==========================================================================
    // CreateActivity Tests (T049)
    // ==========================================================================

    #[test]
    fn test_create_activity_struct() {
        let input = CreateActivity {
            actor_id: Uuid::new_v4(),
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
        };

        assert_eq!(input.activity_type, ActivityType::Create);
        assert_eq!(input.target_type, TargetType::Recipe);
    }

    #[test]
    fn test_create_activity_all_combinations() {
        let actor = Uuid::new_v4();
        let target = Uuid::new_v4();

        // Create recipe
        let create_recipe = CreateActivity {
            actor_id: actor,
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: target,
        };
        assert_eq!(create_recipe.activity_type, ActivityType::Create);

        // Share book
        let share_book = CreateActivity {
            actor_id: actor,
            activity_type: ActivityType::Share,
            target_type: TargetType::Book,
            target_id: target,
        };
        assert_eq!(share_book.activity_type, ActivityType::Share);

        // Follow user
        let follow_user = CreateActivity {
            actor_id: actor,
            activity_type: ActivityType::Follow,
            target_type: TargetType::User,
            target_id: target,
        };
        assert_eq!(follow_user.activity_type, ActivityType::Follow);
    }

    // ==========================================================================
    // ActivityWithActor Tests (T049)
    // ==========================================================================

    #[test]
    fn test_activity_with_actor_serialization() {
        let activity = ActivityWithActor {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            actor_username: "chef_alice".to_string(),
            actor_display_name: "Alice the Chef".to_string(),
            actor_avatar_url: Some("https://example.com/avatars/alice.jpg".to_string()),
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("\"actor_username\":\"chef_alice\""));
        assert!(json.contains("\"actor_display_name\":\"Alice the Chef\""));
    }

    #[test]
    fn test_activity_with_actor_roundtrip() {
        let activity = ActivityWithActor {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            actor_username: "bob".to_string(),
            actor_display_name: "Bob".to_string(),
            actor_avatar_url: None,
            activity_type: ActivityType::Share,
            target_type: TargetType::Book,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&activity).unwrap();
        let deserialized: ActivityWithActor = serde_json::from_str(&json).unwrap();

        assert_eq!(activity.id, deserialized.id);
        assert_eq!(activity.actor_username, deserialized.actor_username);
        assert_eq!(activity.actor_display_name, deserialized.actor_display_name);
        assert_eq!(activity.actor_avatar_url, deserialized.actor_avatar_url);
    }

    #[test]
    fn test_activity_with_actor_no_avatar() {
        let activity = ActivityWithActor {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            actor_username: "minimal".to_string(),
            actor_display_name: "Minimal User".to_string(),
            actor_avatar_url: None,
            activity_type: ActivityType::Follow,
            target_type: TargetType::User,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("\"actor_avatar_url\":null"));
    }

    // ==========================================================================
    // AP ID Format Tests (T049)
    // ==========================================================================

    #[test]
    fn test_activity_ap_id_format() {
        let base_url = "https://oppskrift.example.com";
        let id = Uuid::new_v4();
        let ap_id = format!("{}/activities/{}", base_url, id);

        let activity = Activity {
            id,
            actor_id: Uuid::new_v4(),
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: ap_id.clone(),
        };

        assert!(activity.ap_id.starts_with(base_url));
        assert!(activity.ap_id.contains("/activities/"));
        assert!(activity.ap_id.contains(&id.to_string()));
    }

    // ==========================================================================
    // UUID Uniqueness Tests (T049)
    // ==========================================================================

    #[test]
    fn test_activity_uuid_uniqueness() {
        let activity1 = Activity {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/1".to_string(),
        };

        let activity2 = Activity {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/2".to_string(),
        };

        assert_ne!(activity1.id, activity2.id);
        assert_ne!(activity1.actor_id, activity2.actor_id);
        assert_ne!(activity1.target_id, activity2.target_id);
    }
}
