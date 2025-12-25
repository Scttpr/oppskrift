use sqlx::PgPool;
use uuid::Uuid;

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
                created_at, updated_at, ap_id
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
                created_at, updated_at, ap_id
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User @{} not found", username)))
    }

    /// Create a new user
    pub async fn create(pool: &PgPool, input: CreateUser) -> AppResult<User> {
        let measurement_pref = input.measurement_pref.unwrap_or_default();

        sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (username, display_name, bio, avatar_url, measurement_pref, ap_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id, username, display_name, bio, avatar_url,
                measurement_pref as "measurement_pref: MeasurementPref",
                created_at, updated_at, ap_id
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
            sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("users_username_key") => {
                AppError::Conflict(format!("Username '{}' is already taken", input.username))
            }
            sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("users_ap_id_key") => {
                AppError::Conflict("ActivityPub ID already exists".to_string())
            }
            _ => AppError::from(e),
        })
    }

    /// Update a user's profile
    pub async fn update(pool: &PgPool, id: Uuid, input: UpdateUser) -> AppResult<User> {
        sqlx::query_as!(
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
                created_at, updated_at, ap_id
            "#,
            id,
            input.display_name,
            input.bio,
            input.avatar_url,
            input.measurement_pref as Option<MeasurementPref>
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User {} not found", id)))
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
