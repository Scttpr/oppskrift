use sqlx::PgPool;
use uuid::Uuid;

use crate::core::audit::AuditEvent;
use crate::core::crypto::generate_rsa_keypair;
use crate::core::error::{AppError, AppResult};
use crate::models::user::{CreateUser, UpdateUser, User};

/// Service for user-related database operations
pub struct UserService;

impl UserService {
    /// Get a user by their ID
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT
                id, username, email, email_verified, password_hash,
                display_name, bio, avatar_url, measurement_pref,
                totp_secret_encrypted, totp_enabled,
                failed_login_attempts, locked_until, deletion_requested_at,
                deletion_content_choice, created_at, updated_at, ap_id, federation_enabled
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))
    }

    /// Get a user by their username
    pub async fn get_by_username(pool: &PgPool, username: &str) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT
                id, username, email, email_verified, password_hash,
                display_name, bio, avatar_url, measurement_pref,
                totp_secret_encrypted, totp_enabled,
                failed_login_attempts, locked_until, deletion_requested_at,
                deletion_content_choice, created_at, updated_at, ap_id, federation_enabled
            FROM users
            WHERE username = $1
            "#,
        )
        .bind(username)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User @{} not found", username)))
    }

    /// Get a user by their email
    pub async fn get_by_email(pool: &PgPool, email: &str) -> AppResult<User> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT
                id, username, email, email_verified, password_hash,
                display_name, bio, avatar_url, measurement_pref,
                totp_secret_encrypted, totp_enabled,
                failed_login_attempts, locked_until, deletion_requested_at,
                deletion_content_choice, created_at, updated_at, ap_id, federation_enabled
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }

    /// Create a new user with RSA keypair for ActivityPub federation
    pub async fn create(pool: &PgPool, input: CreateUser) -> AppResult<User> {
        let measurement_pref = input.measurement_pref.unwrap_or_default();

        // Generate RSA keypair for ActivityPub HTTP Signatures
        // RSA-2048 keygen is CPU-bound; run off the async runtime.
        let keypair = tokio::task::spawn_blocking(generate_rsa_keypair)
            .await
            .map_err(|e| AppError::Internal(format!("RSA keygen task failed: {}", e)))??;

        // Create user with auth fields
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, display_name, bio, avatar_url, measurement_pref, ap_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id, username, email, email_verified, password_hash,
                display_name, bio, avatar_url, measurement_pref,
                totp_secret_encrypted, totp_enabled,
                failed_login_attempts, locked_until, deletion_requested_at,
                deletion_content_choice, created_at, updated_at, ap_id, federation_enabled
            "#,
        )
        .bind(&input.username)
        .bind(&input.email)
        .bind(&input.password_hash)
        .bind(&input.display_name)
        .bind(&input.bio)
        .bind(&input.avatar_url)
        .bind(measurement_pref)
        .bind(&input.ap_id)
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err)
                if db_err.constraint() == Some("users_username_key") =>
            {
                AppError::Conflict(format!("Username '{}' is already taken", input.username))
            }
            sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("users_email_key") => {
                AppError::Conflict("Email already registered".to_string())
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
        let user = sqlx::query_as::<_, User>(
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
                id, username, email, email_verified, password_hash,
                display_name, bio, avatar_url, measurement_pref,
                totp_secret_encrypted, totp_enabled,
                failed_login_attempts, locked_until, deletion_requested_at,
                deletion_content_choice, created_at, updated_at, ap_id, federation_enabled
            "#,
        )
        .bind(id)
        .bind(&input.display_name)
        .bind(&input.bio)
        .bind(&input.avatar_url)
        .bind(input.measurement_pref)
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

    /// Check if an email is available
    pub async fn email_available(pool: &PgPool, email: &str) -> AppResult<bool> {
        let exists: Option<bool> =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE email = $1)")
                .bind(email)
                .fetch_one(pool)
                .await?;

        Ok(!exists.unwrap_or(false))
    }

    /// Search users by username prefix (for autocomplete)
    /// Returns up to `limit` users whose username starts with the query
    pub async fn search_by_username(
        pool: &PgPool,
        query: &str,
        limit: i64,
    ) -> AppResult<Vec<User>> {
        // Sanitize and prepare the query for LIKE matching
        let pattern = format!(
            "{}%",
            query
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect::<String>()
        );

        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, email_verified, password_hash,
                   display_name, bio, avatar_url, measurement_pref,
                   totp_secret_encrypted, totp_enabled,
                   failed_login_attempts, locked_until, deletion_requested_at,
                   deletion_content_choice, created_at, updated_at, ap_id, federation_enabled
            FROM users
            WHERE LOWER(username) LIKE $1
            ORDER BY username ASC
            LIMIT $2
            "#,
        )
        .bind(&pattern)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(users)
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

    /// Persist the user's chosen content handling preference for account deletion
    pub async fn set_deletion_content_choice(
        pool: &PgPool,
        user_id: Uuid,
        choice: crate::models::DeletionContentChoice,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE users SET deletion_content_choice = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(choice)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to save content choice: {}", e)))?;

        Ok(())
    }

    /// Update only the federation_enabled flag for a user
    pub async fn update_federation_enabled(
        pool: &PgPool,
        user_id: Uuid,
        enabled: bool,
    ) -> AppResult<()> {
        sqlx::query!(
            "UPDATE users SET federation_enabled = $1, updated_at = NOW() WHERE id = $2",
            enabled,
            user_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Toggle federation for a user
    /// Returns the updated user and optionally a Delete activity to federate
    pub async fn set_federation_enabled(
        pool: &PgPool,
        user_id: Uuid,
        enabled: bool,
    ) -> AppResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET federation_enabled = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, username, email, email_verified, password_hash,
                display_name, bio, avatar_url, measurement_pref,
                totp_secret_encrypted, totp_enabled,
                failed_login_attempts, locked_until, deletion_requested_at,
                deletion_content_choice, created_at, updated_at, ap_id, federation_enabled
            "#,
        )
        .bind(user_id)
        .bind(enabled)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::MeasurementPref;

    #[test]
    fn test_create_user_input() {
        let input = CreateUser {
            username: "chef".to_string(),
            email: "chef@example.com".to_string(),
            password_hash: "hashed_password".to_string(),
            display_name: "Chef Marie".to_string(),
            bio: Some("I love cooking".to_string()),
            avatar_url: None,
            measurement_pref: Some(MeasurementPref::Metric),
            ap_id: "https://example.com/users/chef".to_string(),
        };

        assert_eq!(input.username, "chef");
        assert_eq!(input.email, "chef@example.com");
        assert_eq!(input.measurement_pref, Some(MeasurementPref::Metric));
    }
}
