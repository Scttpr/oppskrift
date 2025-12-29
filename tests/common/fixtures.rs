//! Test fixtures for creating common test entities
//!
//! Provides reusable factory functions for creating test data.

use serde_json::{json, Value};
use uuid::Uuid;

/// Standard test password meeting complexity requirements
pub const TEST_PASSWORD: &str = "Xk9#mP2$vL5@nQ8!";

/// Create a test user registration payload
pub fn create_test_user(suffix: &str) -> Value {
    let id = Uuid::new_v4().to_string().replace('-', "");
    json!({
        "email": format!("test_{}_{suffix}@example.com", &id[..8]),
        "username": format!("test_{}", &id[..12]),
        "password": TEST_PASSWORD,
        "display_name": format!("Test User {suffix}")
    })
}

/// Create a test recipe payload
pub fn create_test_recipe() -> Value {
    json!({
        "title": "Test Recipe",
        "description": "A delicious test recipe",
        "prep_time_min": 15,
        "cook_time_min": 30,
        "servings": "4",
        "difficulty": "medium",
        "visibility": "public",
        "ingredients": [
            {
                "position": 1,
                "name": "Test Ingredient 1",
                "quantity": 2.0,
                "unit": "cups"
            },
            {
                "position": 2,
                "name": "Test Ingredient 2",
                "quantity": 1.0,
                "unit": "tbsp"
            }
        ],
        "instructions": [
            {
                "step_number": 1,
                "description": "First step of the recipe"
            },
            {
                "step_number": 2,
                "description": "Second step of the recipe"
            }
        ]
    })
}

/// Create test ingredients payload
pub fn create_test_ingredients(count: usize) -> Vec<Value> {
    (1..=count)
        .map(|i| {
            json!({
                "name": format!("Ingredient {}", i),
                "quantity": format!("{}", i),
                "unit": "units"
            })
        })
        .collect()
}

/// Create a mock ActivityPub actor
pub fn mock_remote_actor(username: &str, domain: &str) -> Value {
    json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "type": "Person",
        "id": format!("https://{}/users/{}", domain, username),
        "preferredUsername": username,
        "name": format!("Remote User {}", username),
        "inbox": format!("https://{}/users/{}/inbox", domain, username),
        "outbox": format!("https://{}/users/{}/outbox", domain, username),
        "publicKey": {
            "id": format!("https://{}/users/{}#main-key", domain, username),
            "owner": format!("https://{}/users/{}", domain, username),
            "publicKeyPem": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...\n-----END PUBLIC KEY-----"
        }
    })
}

/// Create a mock ActivityPub Follow activity
pub fn mock_follow_activity(actor_id: &str, target_id: &str) -> Value {
    json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "type": "Follow",
        "id": format!("{}/follows/{}", actor_id, Uuid::new_v4()),
        "actor": actor_id,
        "object": target_id
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_user() {
        let user = create_test_user("01");
        assert!(user["email"].as_str().unwrap().contains("@example.com"));
        assert!(user["username"].as_str().unwrap().starts_with("test_"));
        assert_eq!(user["password"], TEST_PASSWORD);
    }

    #[test]
    fn test_create_test_recipe() {
        let recipe = create_test_recipe();
        assert_eq!(recipe["title"], "Test Recipe");
        assert!(recipe["ingredients"].as_array().unwrap().len() >= 2);
        assert!(recipe["instructions"].as_array().unwrap().len() >= 2);
    }

    #[test]
    fn test_create_test_ingredients() {
        let ingredients = create_test_ingredients(5);
        assert_eq!(ingredients.len(), 5);
        assert_eq!(ingredients[0]["name"], "Ingredient 1");
        assert_eq!(ingredients[4]["name"], "Ingredient 5");
    }

    #[test]
    fn test_mock_remote_actor() {
        let actor = mock_remote_actor("alice", "remote.example");
        assert_eq!(actor["type"], "Person");
        assert!(actor["id"]
            .as_str()
            .unwrap()
            .contains("remote.example/users/alice"));
    }
}
