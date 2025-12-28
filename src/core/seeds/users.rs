//! Test user seed data

use sqlx::PgPool;
use uuid::Uuid;

use super::SeedError;
use crate::services::PasswordService;

/// Test user credentials (username, email, password, display_name, bio, measurement_pref)
const TEST_USERS: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "alice",
        "alice@example.com",
        "AlicePass123",
        "Alice Martin",
        "Home cook passionate about French cuisine and baking.",
        "metric",
    ),
    (
        "bob",
        "bob@example.com",
        "BobSecure456",
        "Bob Wilson",
        "BBQ enthusiast from Texas. Love smoking meats!",
        "imperial",
    ),
    (
        "chef_marie",
        "marie@example.com",
        "MarieChef789",
        "Chef Marie Dubois",
        "Professional chef with 15 years experience. Specializing in Mediterranean cuisine.",
        "metric",
    ),
];

/// Seed test users
///
/// Returns vector of created user IDs in order: [alice, bob, chef_marie]
pub async fn seed(pool: &PgPool) -> Result<Vec<Uuid>, SeedError> {
    let password_service = PasswordService::new(false); // Disable HIBP for seeds
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let mut user_ids = Vec::with_capacity(TEST_USERS.len());

    for (username, email, password, display_name, bio, measurement_pref) in TEST_USERS {
        let password_hash = password_service
            .hash(password)
            .map_err(|e| SeedError::Password(e.to_string()))?;

        let ap_id = format!("{}/users/{}", base_url, username);

        let user_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO users (username, email, email_verified, password_hash, display_name, bio, measurement_pref, ap_id)
            VALUES ($1, $2, true, $3, $4, $5, $6::measurement_pref, $7)
            RETURNING id
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(&password_hash)
        .bind(display_name)
        .bind(bio)
        .bind(measurement_pref)
        .bind(&ap_id)
        .fetch_one(pool)
        .await?;

        user_ids.push(user_id);
    }

    Ok(user_ids)
}
