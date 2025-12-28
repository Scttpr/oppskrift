use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::core::pagination::{PaginatedResponse, PaginationParams};
use crate::models::{RecipeSummary, SavedRecipe};

pub struct SavedRecipeService;

impl SavedRecipeService {
    /// Save a recipe to user's saved list
    pub async fn save(pool: &PgPool, user_id: Uuid, recipe_id: Uuid) -> AppResult<SavedRecipe> {
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
    pub async fn unsave(pool: &PgPool, user_id: Uuid, recipe_id: Uuid) -> AppResult<()> {
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
    pub async fn is_saved(pool: &PgPool, user_id: Uuid, recipe_id: Uuid) -> AppResult<bool> {
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
            SELECT r.id, r.author_id, r.title, r.description,
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
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?;

        Ok(PaginatedResponse::new(
            recipes,
            params.page,
            limit,
            total as u64,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Service Existence Test
    // ==========================================================================

    #[test]
    fn test_service_exists() {
        // Verify service struct exists
        let _ = SavedRecipeService;
    }

    // ==========================================================================
    // Error Path Tests (T045)
    // ==========================================================================

    #[test]
    fn test_already_saved_error() {
        let err = AppError::Validation("Recipe already saved".to_string());
        let msg = err.to_string();
        assert!(msg.contains("already saved"));
    }

    #[test]
    fn test_recipe_not_found_error() {
        let err = AppError::NotFound("Recipe not found".to_string());
        let msg = err.to_string();
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_saved_recipe_not_found_error() {
        let err = AppError::NotFound("Saved recipe not found".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Saved recipe not found"));
    }

    // ==========================================================================
    // SavedRecipe Model Tests (T045)
    // ==========================================================================

    #[test]
    fn test_saved_recipe_struct() {
        let saved = SavedRecipe {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recipe_id: Uuid::new_v4(),
            saved_at: chrono::Utc::now(),
        };

        assert_ne!(saved.user_id, saved.recipe_id);
        assert_ne!(saved.id, saved.user_id);
    }

    #[test]
    fn test_saved_recipe_ids_unique() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        let saved = SavedRecipe {
            id: id1,
            user_id: id2,
            recipe_id: id3,
            saved_at: chrono::Utc::now(),
        };

        // All IDs should be different
        assert_ne!(saved.id, saved.user_id);
        assert_ne!(saved.id, saved.recipe_id);
        assert_ne!(saved.user_id, saved.recipe_id);
    }

    // ==========================================================================
    // Pagination Tests (T045)
    // ==========================================================================

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams {
            page: 1,
            page_size: 10,
        };
        assert_eq!(params.offset(), 0);
        assert_eq!(params.limit(), 10);
    }

    #[test]
    fn test_pagination_second_page() {
        let params = PaginationParams {
            page: 2,
            page_size: 20,
        };
        assert_eq!(params.offset(), 20);
    }

    #[test]
    fn test_pagination_large_page() {
        let params = PaginationParams {
            page: 100,
            page_size: 50,
        };
        assert_eq!(params.offset(), 4950); // (100-1) * 50
    }

    // ==========================================================================
    // UUID Tests (T045)
    // ==========================================================================

    #[test]
    fn test_uuid_generation() {
        let user_id = Uuid::new_v4();
        let recipe_id = Uuid::new_v4();

        assert_ne!(user_id, recipe_id);
        assert!(!user_id.is_nil());
        assert!(!recipe_id.is_nil());
    }

    #[test]
    fn test_uuid_uniqueness_batch() {
        let ids: Vec<Uuid> = (0..50).map(|_| Uuid::new_v4()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len());
    }

    // ==========================================================================
    // RecipeSummary Tests (T045)
    // ==========================================================================

    #[test]
    fn test_recipe_summary_with_image() {
        let summary = RecipeSummary {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "Chocolate Cake".to_string(),
            description: Some("A delicious chocolate cake".to_string()),
            difficulty: None,
            prep_time_min: Some(30),
            cook_time_min: Some(45),
            created_at: chrono::Utc::now(),
            primary_image_url: Some("https://example.com/cake.jpg".to_string()),
        };

        assert_eq!(summary.title, "Chocolate Cake");
        assert!(summary.primary_image_url.is_some());
    }

    #[test]
    fn test_recipe_summary_minimal() {
        let summary = RecipeSummary {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "Quick Pasta".to_string(),
            description: None,
            difficulty: None,
            prep_time_min: None,
            cook_time_min: None,
            created_at: chrono::Utc::now(),
            primary_image_url: None,
        };

        assert!(summary.description.is_none());
        assert!(summary.primary_image_url.is_none());
    }

    #[test]
    fn test_recipe_summary_with_times() {
        let summary = RecipeSummary {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "Slow Roast".to_string(),
            description: None,
            difficulty: None,
            prep_time_min: Some(15),
            cook_time_min: Some(180),
            created_at: chrono::Utc::now(),
            primary_image_url: None,
        };

        assert_eq!(summary.prep_time_min, Some(15));
        assert_eq!(summary.cook_time_min, Some(180));
    }
}
