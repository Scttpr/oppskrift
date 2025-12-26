use sqlx::PgPool;
use uuid::Uuid;

use crate::lib::audit::AuditEvent;
use crate::lib::crypto::generate_rsa_keypair;
use crate::lib::error::{AppError, AppResult};
use crate::models::user::{CreateUser, MeasurementPref, UpdateUser, User};

/// Service for user-related database operations
pub struct UserService;

impl UserService {
    /// Get a user by their ID
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> AppResult<User> {
        sqlx::query_as!(
            User,
            r#"
            SELECT
                id, username, display_name, bio, avatar_url,
                measurement_pref as "measurement_pref: MeasurementPref",
                created_at, updated_at, ap_id, federation_enabled
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))
    }

    /// Get a user by their username
    pub async fn get_by_username(pool: &PgPool, username: &str) -> AppResult<User> {
        sqlx::query_as!(
            User,
            r#"
            SELECT
                id, username, display_name, bio, avatar_url,
                measurement_pref as "measurement_pref: MeasurementPref",
                created_at, updated_at, ap_id, federation_enabled
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User @{} not found", username)))
    }

    /// Create a new user with RSA keypair for ActivityPub federation
    pub async fn create(pool: &PgPool, input: CreateUser) -> AppResult<User> {
        let measurement_pref = input.measurement_pref.unwrap_or_default();

        // Generate RSA keypair for ActivityPub HTTP Signatures
        let keypair = generate_rsa_keypair()?;

        // Create user
        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username, display_name, bio, avatar_url, measurement_pref, ap_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id, username, display_name, bio, avatar_url,
                measurement_pref as "measurement_pref: MeasurementPref",
                created_at, updated_at, ap_id, federation_enabled
            "#,
            input.username,
            input.display_name,
            input.bio,
            input.avatar_url,
            measurement_pref as MeasurementPref,
            input.ap_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err)
                if db_err.constraint() == Some("users_username_key") =>
            {
                AppError::Conflict(format!("Username '{}' is already taken", input.username))
            }
            sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("users_ap_id_key") => {
                AppError::Conflict("ActivityPub ID already exists".to_string())
            }
            _ => AppError::from(e),
        })?;

        // Store RSA keypair for the user
        sqlx::query!(
            r#"
            INSERT INTO user_keys (user_id, public_key_pem, private_key_pem)
            VALUES ($1, $2, $3)
            "#,
            user.id,
            keypair.public_key_pem,
            keypair.private_key_pem
        )
        .execute(pool)
        .await?;

        // Audit user creation
        AuditEvent::new("user.create")
            .with_user(user.id)
            .with_target("user", user.id)
            .log();

        Ok(user)
    }

    /// Update a user's profile
    pub async fn update(pool: &PgPool, id: Uuid, input: UpdateUser) -> AppResult<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET
                display_name = COALESCE($2, display_name),
                bio = COALESCE($3, bio),
                avatar_url = COALESCE($4, avatar_url),
                measurement_pref = COALESCE($5, measurement_pref),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, username, display_name, bio, avatar_url,
                measurement_pref as "measurement_pref: MeasurementPref",
                created_at, updated_at, ap_id, federation_enabled
            "#,
            id,
            input.display_name,
            input.bio,
            input.avatar_url,
            input.measurement_pref as Option<MeasurementPref>
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))?;

        // Audit user update
        AuditEvent::new("user.update")
            .with_user(id)
            .with_target("user", id)
            .log();

        Ok(user)
    }

    /// Check if a username is available
    pub async fn username_available(pool: &PgPool, username: &str) -> AppResult<bool> {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)",
            username
        )
        .fetch_one(pool)
        .await?;

        Ok(!exists.unwrap_or(false))
    }

    /// Get the public key PEM for a user (for ActivityPub Actor profile)
    pub async fn get_public_key(pool: &PgPool, user_id: Uuid) -> AppResult<String> {
        let key = sqlx::query_scalar!(
            "SELECT public_key_pem FROM user_keys WHERE user_id = $1",
            user_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Keys not found for user {}", user_id)))?;

        Ok(key)
    }

    /// Get the private key PEM for a user (for signing HTTP requests)
    pub async fn get_private_key(pool: &PgPool, user_id: Uuid) -> AppResult<String> {
        let key = sqlx::query_scalar!(
            "SELECT private_key_pem FROM user_keys WHERE user_id = $1",
            user_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Keys not found for user {}", user_id)))?;

        Ok(key)
    }

    /// Toggle federation for a user
    /// Returns the updated user and optionally a Delete activity to federate
    pub async fn set_federation_enabled(
        pool: &PgPool,
        user_id: Uuid,
        enabled: bool,
    ) -> AppResult<User> {
        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET federation_enabled = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, username, display_name, bio, avatar_url,
                measurement_pref as "measurement_pref: MeasurementPref",
                created_at, updated_at, ap_id, federation_enabled
            "#,
            user_id,
            enabled
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))?;

        // Audit federation toggle
        AuditEvent::new("user.federation.toggle")
            .with_user(user_id)
            .with_metadata("enabled", &enabled.to_string())
            .log();

        Ok(user)
    }

    /// Build a Delete activity for when a user disables federation
    /// Caller should deliver this to known followers
    pub fn build_delete_activity(user_id: Uuid) -> crate::lib::activitypub::Activity {
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        crate::lib::activitypub::Activity::delete_actor(&base_url, user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user_input() {
        let input = CreateUser {
            username: "chef".to_string(),
            display_name: "Chef Marie".to_string(),
            bio: Some("I love cooking".to_string()),
            avatar_url: None,
            measurement_pref: Some(MeasurementPref::Metric),
            ap_id: "https://example.com/users/chef".to_string(),
        };

        assert_eq!(input.username, "chef");
        assert_eq!(input.measurement_pref, Some(MeasurementPref::Metric));
    }
}
