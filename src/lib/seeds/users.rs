//! User seed data

use sqlx::PgPool;
use uuid::Uuid;

use crate::lib::error::AppResult;
use crate::models::{CreateUser, MeasurementPref};
use crate::services::UserService;

/// Test user data
struct TestUser {
    username: &'static str,
    display_name: &'static str,
    bio: &'static str,
    measurement_pref: MeasurementPref,
}

const TEST_USERS: &[TestUser] = &[
    TestUser {
        username: "alice",
        display_name: "Alice Chen",
        bio: "Home cook passionate about Asian fusion cuisine. Love experimenting with traditional recipes!",
        measurement_pref: MeasurementPref::Metric,
    },
    TestUser {
        username: "bob",
        display_name: "Bob Wilson",
        bio: "BBQ enthusiast from Texas. Smoking meats is my meditation.",
        measurement_pref: MeasurementPref::Imperial,
    },
    TestUser {
        username: "chef_marie",
        display_name: "Chef Marie Dubois",
        bio: "Professional pastry chef with 15 years experience. Currently teaching at culinary school.",
        measurement_pref: MeasurementPref::Metric,
    },
];

/// Seed test users
pub async fn seed_users(pool: &PgPool, base_url: &str) -> AppResult<Vec<Uuid>> {
    let mut user_ids = Vec::new();

    for user in TEST_USERS {
        // Check if user already exists
        if UserService::get_by_username(pool, user.username)
            .await
            .is_ok()
        {
            tracing::debug!("User {} already exists, skipping", user.username);
            continue;
        }

        let input = CreateUser {
            username: user.username.to_string(),
            display_name: user.display_name.to_string(),
            bio: Some(user.bio.to_string()),
            avatar_url: None,
            measurement_pref: Some(user.measurement_pref),
            ap_id: format!("{}/users/{}", base_url, user.username),
        };

        let created = UserService::create(pool, input).await?;
        user_ids.push(created.id);
        tracing::debug!("Created user: {} ({})", user.username, created.id);
    }

    Ok(user_ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_users_defined() {
        assert_eq!(TEST_USERS.len(), 3);
        assert_eq!(TEST_USERS[0].username, "alice");
        assert_eq!(TEST_USERS[1].measurement_pref, MeasurementPref::Imperial);
    }
}
