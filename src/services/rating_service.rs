use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::models::RatingSummary;

/// Service for recipe star ratings
pub struct RatingService;

impl RatingService {
    /// Set (or update) the current user's rating for a recipe. Value must be 1–5.
    pub async fn set_rating(
        pool: &PgPool,
        recipe_id: Uuid,
        user_id: Uuid,
        value: i16,
    ) -> AppResult<()> {
        if !(1..=5).contains(&value) {
            return Err(AppError::Validation(
                "Rating must be between 1 and 5".to_string(),
            ));
        }

        sqlx::query!(
            r#"
            INSERT INTO recipe_ratings (recipe_id, user_id, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (recipe_id, user_id)
            DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()
            "#,
            recipe_id,
            user_id,
            value
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Remove the current user's rating for a recipe (no-op if none exists).
    pub async fn delete_rating(pool: &PgPool, recipe_id: Uuid, user_id: Uuid) -> AppResult<()> {
        sqlx::query!(
            "DELETE FROM recipe_ratings WHERE recipe_id = $1 AND user_id = $2",
            recipe_id,
            user_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get the aggregate rating summary for a recipe, including the viewer's
    /// own rating if they are logged in.
    pub async fn get_summary(
        pool: &PgPool,
        recipe_id: Uuid,
        viewer_id: Option<Uuid>,
    ) -> AppResult<RatingSummary> {
        let agg = sqlx::query!(
            r#"
            SELECT
                AVG(value)::float8 as "average?",
                COUNT(*) as "count!"
            FROM recipe_ratings
            WHERE recipe_id = $1
            "#,
            recipe_id
        )
        .fetch_one(pool)
        .await?;

        let user_rating = match viewer_id {
            Some(uid) => {
                sqlx::query_scalar!(
                    "SELECT value FROM recipe_ratings WHERE recipe_id = $1 AND user_id = $2",
                    recipe_id,
                    uid
                )
                .fetch_optional(pool)
                .await?
            }
            None => None,
        };

        Ok(RatingSummary {
            average: agg.average,
            count: agg.count,
            user_rating,
        })
    }
}
