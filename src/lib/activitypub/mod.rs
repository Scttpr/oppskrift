//! ActivityPub federation support
//!
//! Implements ActivityPub protocol for federated recipe sharing.
//! See: https://www.w3.org/TR/activitypub/

pub mod actor;
pub mod objects;
pub mod signature;

pub use actor::*;
pub use objects::*;
pub use signature::*;

use serde::{Deserialize, Serialize};

/// ActivityPub context URLs
pub const ACTIVITYSTREAMS_CONTEXT: &str = "https://www.w3.org/ns/activitystreams";
pub const SECURITY_CONTEXT: &str = "https://w3id.org/security/v1";

/// Common ActivityPub activity types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActivityType {
    Create,
    Update,
    Delete,
    Follow,
    Accept,
    Reject,
    Undo,
    Announce,
    Like,
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityType::Create => write!(f, "Create"),
            ActivityType::Update => write!(f, "Update"),
            ActivityType::Delete => write!(f, "Delete"),
            ActivityType::Follow => write!(f, "Follow"),
            ActivityType::Accept => write!(f, "Accept"),
            ActivityType::Reject => write!(f, "Reject"),
            ActivityType::Undo => write!(f, "Undo"),
            ActivityType::Announce => write!(f, "Announce"),
            ActivityType::Like => write!(f, "Like"),
        }
    }
}

/// Base Activity structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    pub id: String,
    #[serde(rename = "type")]
    pub activity_type: String,
    pub actor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
}

impl Activity {
    /// Create a new activity with default context
    pub fn new(id: String, activity_type: ActivityType, actor: String) -> Self {
        Self {
            context: serde_json::json!([
                ACTIVITYSTREAMS_CONTEXT,
                SECURITY_CONTEXT
            ]),
            id,
            activity_type: activity_type.to_string(),
            actor,
            object: None,
            target: None,
            published: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Set the object of this activity
    pub fn with_object(mut self, object: serde_json::Value) -> Self {
        self.object = Some(object);
        self
    }

    /// Set the target of this activity
    pub fn with_target(mut self, target: String) -> Self {
        self.target = Some(target);
        self
    }

    /// Create a Delete activity for an actor (used when disabling federation)
    pub fn delete_actor(base_url: &str, actor_id: uuid::Uuid) -> Self {
        let actor_url = format!("{}/users/{}", base_url, actor_id);
        let activity_id = format!("{}/activities/{}", base_url, uuid::Uuid::new_v4());

        Self::new(activity_id, ActivityType::Delete, actor_url.clone())
            .with_object(serde_json::json!({
                "id": actor_url,
                "type": "Tombstone"
            }))
    }

    /// Create a Delete activity for a recipe
    pub fn delete_recipe(base_url: &str, actor_id: uuid::Uuid, recipe_id: uuid::Uuid) -> Self {
        let actor_url = format!("{}/users/{}", base_url, actor_id);
        let recipe_url = format!("{}/recipes/{}", base_url, recipe_id);
        let activity_id = format!("{}/activities/{}", base_url, uuid::Uuid::new_v4());

        Self::new(activity_id, ActivityType::Delete, actor_url)
            .with_object(serde_json::json!({
                "id": recipe_url,
                "type": "Tombstone"
            }))
    }
}
