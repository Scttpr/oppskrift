use sqlx::PgPool;
use uuid::Uuid;

use crate::lib::error::{AppError, AppResult};
use crate::lib::pagination::{PaginatedResponse, PaginationParams};
use crate::models::{RecipeSummary, SavedRecipe};

pub struct SavedRecipeService;

impl SavedRecipeService {
    /// Save a recipe to user's saved list
    pub async fn save(
        pool: &PgPool,
        user_id: Uuid,
        recipe_id: Uuid,
    ) -> AppResult<SavedRecipe> {
        let id = Uuid::new_v4();

        sqlx::query_as!(
            SavedRecipe,
            r#"
            INSERT INTO saved_recipes (id, user_id, recipe_id)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, recipe_id, saved_at
            "#,
            id,
            user_id,
            recipe_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err)
                if db_err.constraint() == Some("saved_recipes_unique") =>
            {
                AppError::Validation("Recipe already saved".to_string())
            }
            sqlx::Error::Database(ref db_err)
                if db_err.constraint() == Some("saved_recipes_recipe_id_fkey") =>
            {
                AppError::NotFound("Recipe not found".to_string())
            }
            _ => AppError::from(e),
        })
    }

    /// Remove a recipe from user's saved list
    pub async fn unsave(
        pool: &PgPool,
        user_id: Uuid,
        recipe_id: Uuid,
    ) -> AppResult<()> {
        let result = sqlx::query!(
            r#"
            DELETE FROM saved_recipes
            WHERE user_id = $1 AND recipe_id = $2
            "#,
            user_id,
            recipe_id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Saved recipe not found".to_string()));
        }

        Ok(())
    }

    /// Check if a user has saved a recipe
    pub async fn is_saved(
        pool: &PgPool,
        user_id: Uuid,
        recipe_id: Uuid,
    ) -> AppResult<bool> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM saved_recipes
                WHERE user_id = $1 AND recipe_id = $2
            ) as "exists!"
            "#,
            user_id,
            recipe_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Get user's saved recipes with pagination
    pub async fn get_saved(
        pool: &PgPool,
        user_id: Uuid,
        params: &PaginationParams,
    ) -> AppResult<PaginatedResponse<RecipeSummary>> {
        let limit = params.limit();
        let offset = params.offset();

        // Get total count
        let total: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM saved_recipes sr
            INNER JOIN recipes r ON r.id = sr.recipe_id
            WHERE sr.user_id = $1
            "#,
            user_id
        )
        .fetch_one(pool)
        .await?;

        // Get paginated recipes
        let recipes = sqlx::query_as!(
            RecipeSummary,
            r#"
            SELECT r.id, r.title, r.description,
                   r.difficulty as "difficulty: _",
                   r.prep_time_min, r.cook_time_min,
                   r.created_at,
                   (SELECT url FROM recipe_images WHERE recipe_id = r.id AND is_primary = true LIMIT 1) as primary_image_url
            FROM recipes r
            INNER JOIN saved_recipes sr ON sr.recipe_id = r.id
            WHERE sr.user_id = $1
            ORDER BY sr.saved_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit,
            offset
        )
        .fetch_all(pool)
        .await?;

        Ok(PaginatedResponse::new(recipes, total, params))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_exists() {
        // Verify service struct exists
        let _ = SavedRecipeService;
    }
}
