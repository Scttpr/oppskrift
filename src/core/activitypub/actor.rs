//! ActivityPub Actor implementation for User
//!
//! Represents a user as an ActivityPub Person actor.

use serde::{Deserialize, Serialize};

use super::{ACTIVITYSTREAMS_CONTEXT, SECURITY_CONTEXT};
use crate::models::User;

/// ActivityPub Person actor representing a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonActor {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    pub id: String,
    #[serde(rename = "type")]
    pub actor_type: String,
    #[serde(rename = "preferredUsername")]
    pub preferred_username: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<ActorIcon>,
    #[serde(rename = "publicKey")]
    pub public_key: PublicKey,
    pub endpoints: ActorEndpoints,
}

/// Actor icon (avatar)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorIcon {
    #[serde(rename = "type")]
    pub icon_type: String,
    #[serde(rename = "mediaType")]
    pub media_type: String,
    pub url: String,
}

/// Public key for HTTP signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKey {
    pub id: String,
    pub owner: String,
    #[serde(rename = "publicKeyPem")]
    pub public_key_pem: String,
}

/// Actor endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorEndpoints {
    #[serde(rename = "sharedInbox")]
    pub shared_inbox: String,
}

impl PersonActor {
    /// Create a PersonActor from a User
    pub fn from_user(user: &User, base_url: &str, public_key_pem: &str) -> Self {
        let actor_url = format!("{}/users/{}", base_url, user.id);

        Self {
            context: serde_json::json!([ACTIVITYSTREAMS_CONTEXT, SECURITY_CONTEXT]),
            id: actor_url.clone(),
            actor_type: "Person".to_string(),
            preferred_username: user.username.clone(),
            name: user.display_name.clone(),
            summary: user.bio.clone(),
            inbox: format!("{}/inbox", actor_url),
            outbox: format!("{}/outbox", actor_url),
            followers: format!("{}/followers", actor_url),
            following: format!("{}/following", actor_url),
            icon: user.avatar_url.as_ref().map(|url| ActorIcon {
                icon_type: "Image".to_string(),
                media_type: "image/jpeg".to_string(),
                url: url.clone(),
            }),
            public_key: PublicKey {
                id: format!("{}#main-key", actor_url),
                owner: actor_url.clone(),
                public_key_pem: public_key_pem.to_string(),
            },
            endpoints: ActorEndpoints {
                shared_inbox: format!("{}/inbox", base_url),
            },
        }
    }
}

/// WebFinger resource for actor discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFingerResource {
    pub subject: String,
    pub aliases: Vec<String>,
    pub links: Vec<WebFingerLink>,
}

/// WebFinger link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFingerLink {
    pub rel: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub link_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

impl WebFingerResource {
    /// Create a WebFinger resource for a user
    pub fn for_user(username: &str, domain: &str, base_url: &str, user_id: uuid::Uuid) -> Self {
        let actor_url = format!("{}/users/{}", base_url, user_id);
        let acct = format!("acct:{}@{}", username, domain);

        Self {
            subject: acct.clone(),
            aliases: vec![actor_url.clone()],
            links: vec![
                WebFingerLink {
                    rel: "self".to_string(),
                    link_type: Some("application/activity+json".to_string()),
                    href: Some(actor_url),
                    template: None,
                },
                WebFingerLink {
                    rel: "http://webfinger.net/rel/profile-page".to_string(),
                    link_type: Some("text/html".to_string()),
                    href: Some(format!("{}/users/{}", base_url, user_id)),
                    template: None,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_webfinger_resource() {
        let resource =
            WebFingerResource::for_user("alice", "example.com", "https://example.com", Uuid::nil());

        assert_eq!(resource.subject, "acct:alice@example.com");
        assert!(!resource.links.is_empty());
    }
}
