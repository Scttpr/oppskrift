//! Export service
//!
//! Provides methods for GDPR-compliant data export operations.
//! Handles rate limiting, locking, and data collection for user exports.

use chrono::{Duration, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppError;
use crate::models::User;
use crate::services::{FollowService, SecurityEventService};

/// Exported recipe data
#[derive(Debug, Serialize)]
pub struct ExportedRecipe {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub servings: Option<String>,
    pub visibility: Option<String>,
    pub created_at: String,
}

/// Exported book data
#[derive(Debug, Serialize)]
pub struct ExportedBook {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub visibility: Option<String>,
    pub created_at: String,
}

/// Complete user data export
#[derive(Debug, Serialize)]
pub struct UserDataExport {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub generator: String,
    pub exported_at: String,
    pub profile: serde_json::Value,
    pub recipes: Vec<ExportedRecipe>,
    pub books: Vec<ExportedBook>,
    pub follower_count: i64,
    pub following_count: i64,
}

/// Export rate limit check result
pub enum ExportRateLimitResult {
    /// Export is allowed
    Allowed,
    /// Rate limited - contains minutes to wait
    RateLimited(i64),
}

/// Export service for GDPR data export
pub struct ExportService;

impl ExportService {
    /// Try to acquire an advisory lock for export (prevents concurrent exports)
    ///
    /// Returns true if lock was acquired, false if another export is in progress.
    pub async fn try_acquire_lock(db: &PgPool, user_id: Uuid) -> Result<bool, AppError> {
        let lock_key = user_id.as_u128() as i64;
        let lock_acquired: bool =
            sqlx::query_scalar!("SELECT pg_try_advisory_xact_lock($1)", lock_key)
                .fetch_one(db)
                .await?
                .unwrap_or(false);

        Ok(lock_acquired)
    }

    /// Check if export is rate limited (1 per hour)
    pub async fn check_rate_limit(
        db: &PgPool,
        user_id: Uuid,
    ) -> Result<ExportRateLimitResult, AppError> {
        let last_export = SecurityEventService::get_last_export_time(db, user_id).await?;

        if let Some(last) = last_export {
            let one_hour_ago = Utc::now() - Duration::hours(1);
            if last > one_hour_ago {
                let wait_mins = ((last - one_hour_ago).num_minutes() + 1).max(1);
                return Ok(ExportRateLimitResult::RateLimited(wait_mins));
            }
        }

        Ok(ExportRateLimitResult::Allowed)
    }

    /// Count recipes for a user (for async threshold check)
    pub async fn count_user_recipes(db: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
        let count: i64 =
            sqlx::query_scalar!("SELECT COUNT(*) FROM recipes WHERE author_id = $1", user_id)
                .fetch_one(db)
                .await?
                .unwrap_or(0);

        Ok(count)
    }

    /// Export all recipes for a user
    pub async fn export_recipes(
        db: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<ExportedRecipe>, AppError> {
        let recipes = sqlx::query!(
            r#"
            SELECT id, title, description, prep_time_min, cook_time_min,
                   servings, visibility::text as visibility, created_at
            FROM recipes WHERE author_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|r| ExportedRecipe {
            id: r.id,
            title: r.title,
            description: r.description,
            prep_time_min: r.prep_time_min,
            cook_time_min: r.cook_time_min,
            servings: r.servings,
            visibility: r.visibility,
            created_at: r.created_at.to_rfc3339(),
        })
        .collect();

        Ok(recipes)
    }

    /// Export all books for a user
    pub async fn export_books(db: &PgPool, user_id: Uuid) -> Result<Vec<ExportedBook>, AppError> {
        let books = sqlx::query!(
            r#"
            SELECT id, title, description, visibility::text as visibility, created_at
            FROM recipe_books WHERE owner_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|b| ExportedBook {
            id: b.id,
            title: b.title,
            description: b.description,
            visibility: b.visibility,
            created_at: b.created_at.to_rfc3339(),
        })
        .collect();

        Ok(books)
    }

    /// Build complete user data export
    pub async fn build_export(db: &PgPool, user: &User) -> Result<UserDataExport, AppError> {
        let recipes = Self::export_recipes(db, user.id).await?;
        let books = Self::export_books(db, user.id).await?;
        let follow_counts = FollowService::get_counts(db, user.id).await?;

        let profile = serde_json::json!({
            "id": user.id,
            "username": user.username,
            "display_name": user.display_name,
            "bio": user.bio,
            "avatar_url": user.avatar_url,
            "measurement_pref": format!("{:?}", user.measurement_pref),
            "federation_enabled": user.federation_enabled,
            "created_at": user.created_at.to_rfc3339(),
        });

        Ok(UserDataExport {
            context: "https://www.w3.org/ns/activitystreams".to_string(),
            type_: "OrderedCollection".to_string(),
            generator: "Oppskrift".to_string(),
            exported_at: Utc::now().to_rfc3339(),
            profile,
            recipes,
            books,
            follower_count: follow_counts.followers_count,
            following_count: follow_counts.following_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_rate_limit_result() {
        match ExportRateLimitResult::Allowed {
            ExportRateLimitResult::Allowed => assert!(true),
            _ => panic!("Expected Allowed"),
        }

        match ExportRateLimitResult::RateLimited(30) {
            ExportRateLimitResult::RateLimited(mins) => assert_eq!(mins, 30),
            _ => panic!("Expected RateLimited"),
        }
    }
}
