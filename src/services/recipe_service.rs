use sqlx::PgPool;
use uuid::Uuid;

use crate::lib::audit::AuditEvent;
use crate::lib::error::{AppError, AppResult};
use crate::lib::pagination::{PaginatedResponse, PaginationParams};
use crate::models::{
    CreateIngredient, CreateInstructionStep, CreateRecipe, Difficulty, Ingredient,
    InstructionStep, Recipe, RecipeSummary, UpdateRecipe, Visibility,
};

/// Maximum number of ingredients per recipe
pub const MAX_INGREDIENTS: usize = 50;

/// Maximum number of instruction steps per recipe
pub const MAX_INSTRUCTIONS: usize = 30;

/// Service for recipe-related database operations
pub struct RecipeService;

impl RecipeService {
    /// Validate ingredient count
    pub fn validate_ingredients(ingredients: &[CreateIngredient]) -> AppResult<()> {
        if ingredients.len() > MAX_INGREDIENTS {
            return Err(AppError::Validation(format!(
                "Recipe cannot have more than {} ingredients (got {})",
                MAX_INGREDIENTS,
                ingredients.len()
            )));
        }
        Ok(())
    }

    /// Validate instruction step count
    pub fn validate_instructions(steps: &[CreateInstructionStep]) -> AppResult<()> {
        if steps.len() > MAX_INSTRUCTIONS {
            return Err(AppError::Validation(format!(
                "Recipe cannot have more than {} instruction steps (got {})",
                MAX_INSTRUCTIONS,
                steps.len()
            )));
        }
        Ok(())
    }

    /// Create a new recipe
    pub async fn create(
        pool: &PgPool,
        author_id: Uuid,
        input: CreateRecipe,
        base_url: &str,
    ) -> AppResult<Recipe> {
        let id = Uuid::new_v4();
        let ap_id = format!("{}/recipes/{}", base_url, id);
        let visibility = input.visibility.unwrap_or_default();

        let recipe = sqlx::query_as!(
            Recipe,
            r#"
            INSERT INTO recipes (id, author_id, title, description, visibility, prep_time_min, cook_time_min, servings, difficulty, ap_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING
                id, author_id, title, description,
                visibility as "visibility: Visibility",
                prep_time_min, cook_time_min, servings,
                difficulty as "difficulty: Difficulty",
                created_at, updated_at, ap_id
            "#,
            id,
            author_id,
            input.title,
            input.description,
            visibility as Visibility,
            input.prep_time_min,
            input.cook_time_min,
            input.servings,
            input.difficulty as Option<Difficulty>,
            ap_id
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::from)?;

        // Audit recipe creation
        AuditEvent::new("recipe.create")
            .with_user(author_id)
            .with_target("recipe", recipe.id)
            .log();

        Ok(recipe)
    }

    /// Get a recipe by ID (internal, no visibility check)
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> AppResult<Recipe> {
        sqlx::query_as!(
            Recipe,
            r#"
            SELECT
                id, author_id, title, description,
                visibility as "visibility: Visibility",
                prep_time_min, cook_time_min, servings,
                difficulty as "difficulty: Difficulty",
                created_at, updated_at, ap_id
            FROM recipes
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Recipe {} not found", id)))
    }

    /// Get a recipe by ID with visibility check
    /// Returns 404 for private recipes if viewer is not the author
    pub async fn get_by_id_authorized(
        pool: &PgPool,
        id: Uuid,
        viewer_id: Option<Uuid>,
    ) -> AppResult<Recipe> {
        let recipe = Self::get_by_id(pool, id).await?;

        // Check visibility: private recipes only visible to author
        if recipe.visibility == Visibility::Private {
            if viewer_id != Some(recipe.author_id) {
                return Err(AppError::NotFound(format!("Recipe {} not found", id)));
            }
        }

        Ok(recipe)
    }

    /// Update a recipe
    pub async fn update(pool: &PgPool, id: Uuid, input: UpdateRecipe) -> AppResult<Recipe> {
        let recipe = sqlx::query_as!(
            Recipe,
            r#"
            UPDATE recipes
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                visibility = COALESCE($4, visibility),
                prep_time_min = COALESCE($5, prep_time_min),
                cook_time_min = COALESCE($6, cook_time_min),
                servings = COALESCE($7, servings),
                difficulty = COALESCE($8, difficulty),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, author_id, title, description,
                visibility as "visibility: Visibility",
                prep_time_min, cook_time_min, servings,
                difficulty as "difficulty: Difficulty",
                created_at, updated_at, ap_id
            "#,
            id,
            input.title,
            input.description,
            input.visibility as Option<Visibility>,
            input.prep_time_min,
            input.cook_time_min,
            input.servings,
            input.difficulty as Option<Difficulty>
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Recipe {} not found", id)))?;

        // Audit recipe update
        AuditEvent::new("recipe.update")
            .with_user(recipe.author_id)
            .with_target("recipe", id)
            .log();

        Ok(recipe)
    }

    /// Delete a recipe
    pub async fn delete(pool: &PgPool, id: Uuid, actor_id: Uuid) -> AppResult<()> {
        let result = sqlx::query!("DELETE FROM recipes WHERE id = $1", id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Recipe {} not found", id)));
        }

        // Audit recipe deletion
        AuditEvent::new("recipe.delete")
            .with_user(actor_id)
            .with_target("recipe", id)
            .log();

        Ok(())
    }

    /// List recipes by author
    pub async fn list_by_author(
        pool: &PgPool,
        author_id: Uuid,
        params: &PaginationParams,
    ) -> AppResult<PaginatedResponse<RecipeSummary>> {
        let limit = params.limit();
        let offset = params.offset();

        let recipes = sqlx::query_as!(
            RecipeSummary,
            r#"
            SELECT
                r.id, r.author_id, r.title, r.description,
                r.prep_time_min, r.cook_time_min,
                r.difficulty as "difficulty: Difficulty",
                r.created_at,
                ri.url as primary_image_url
            FROM recipes r
            LEFT JOIN recipe_images ri ON ri.recipe_id = r.id AND ri.is_primary = true
            WHERE r.author_id = $1
            ORDER BY r.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            author_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM recipes WHERE author_id = $1",
            author_id
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok(PaginatedResponse::new(
            recipes,
            params.page,
            limit,
            total as u64,
        ))
    }

    /// List public recipes
    pub async fn list_public(
        pool: &PgPool,
        params: &PaginationParams,
    ) -> AppResult<PaginatedResponse<RecipeSummary>> {
        let limit = params.limit();
        let offset = params.offset();

        let recipes = sqlx::query_as!(
            RecipeSummary,
            r#"
            SELECT
                r.id, r.author_id, r.title, r.description,
                r.prep_time_min, r.cook_time_min,
                r.difficulty as "difficulty: Difficulty",
                r.created_at,
                ri.url as primary_image_url
            FROM recipes r
            LEFT JOIN recipe_images ri ON ri.recipe_id = r.id AND ri.is_primary = true
            WHERE r.visibility = 'public'
            ORDER BY r.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM recipes WHERE visibility = 'public'"
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok(PaginatedResponse::new(
            recipes,
            params.page,
            limit,
            total as u64,
        ))
    }

    /// Get ingredients for a recipe
    pub async fn get_ingredients(pool: &PgPool, recipe_id: Uuid) -> AppResult<Vec<Ingredient>> {
        let ingredients = sqlx::query_as!(
            Ingredient,
            r#"
            SELECT id, recipe_id, position, quantity, unit, name, notes
            FROM ingredients
            WHERE recipe_id = $1
            ORDER BY position
            "#,
            recipe_id
        )
        .fetch_all(pool)
        .await?;

        Ok(ingredients)
    }

    /// Get instruction steps for a recipe
    pub async fn get_instructions(
        pool: &PgPool,
        recipe_id: Uuid,
    ) -> AppResult<Vec<InstructionStep>> {
        let steps = sqlx::query_as!(
            InstructionStep,
            r#"
            SELECT id, recipe_id, step_number, description, image_url, duration_min
            FROM instruction_steps
            WHERE recipe_id = $1
            ORDER BY step_number
            "#,
            recipe_id
        )
        .fetch_all(pool)
        .await?;

        Ok(steps)
    }

    /// Add ingredients to a recipe (batch)
    pub async fn add_ingredients(
        pool: &PgPool,
        recipe_id: Uuid,
        ingredients: Vec<CreateIngredient>,
    ) -> AppResult<Vec<Ingredient>> {
        Self::validate_ingredients(&ingredients)?;

        let mut result = Vec::new();

        for ingredient in ingredients {
            let created = sqlx::query_as!(
                Ingredient,
                r#"
                INSERT INTO ingredients (recipe_id, position, quantity, unit, name, notes)
                VALUES ($1, $2, $3, $4, $5, $6)
                RETURNING id, recipe_id, position, quantity, unit, name, notes
                "#,
                recipe_id,
                ingredient.position,
                ingredient.quantity,
                ingredient.unit,
                ingredient.name,
                ingredient.notes
            )
            .fetch_one(pool)
            .await?;

            result.push(created);
        }

        Ok(result)
    }

    /// Add instruction steps to a recipe (batch)
    pub async fn add_instructions(
        pool: &PgPool,
        recipe_id: Uuid,
        steps: Vec<CreateInstructionStep>,
    ) -> AppResult<Vec<InstructionStep>> {
        Self::validate_instructions(&steps)?;

        let mut result = Vec::new();

        for step in steps {
            let created = sqlx::query_as!(
                InstructionStep,
                r#"
                INSERT INTO instruction_steps (recipe_id, step_number, description, image_url, duration_min)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id, recipe_id, step_number, description, image_url, duration_min
                "#,
                recipe_id,
                step.step_number,
                step.description,
                step.image_url,
                step.duration_min
            )
            .fetch_one(pool)
            .await?;

            result.push(created);
        }

        Ok(result)
    }

    /// Replace all ingredients for a recipe
    pub async fn replace_ingredients(
        pool: &PgPool,
        recipe_id: Uuid,
        ingredients: Vec<CreateIngredient>,
    ) -> AppResult<Vec<Ingredient>> {
        // Delete existing ingredients
        sqlx::query!("DELETE FROM ingredients WHERE recipe_id = $1", recipe_id)
            .execute(pool)
            .await?;

        // Add new ingredients
        Self::add_ingredients(pool, recipe_id, ingredients).await
    }

    /// Replace all instruction steps for a recipe
    pub async fn replace_instructions(
        pool: &PgPool,
        recipe_id: Uuid,
        steps: Vec<CreateInstructionStep>,
    ) -> AppResult<Vec<InstructionStep>> {
        // Delete existing steps
        sqlx::query!(
            "DELETE FROM instruction_steps WHERE recipe_id = $1",
            recipe_id
        )
        .execute(pool)
        .await?;

        // Add new steps
        Self::add_instructions(pool, recipe_id, steps).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_offset_calculation() {
        let params = PaginationParams {
            page: 1,
            page_size: 10,
        };
        assert_eq!(params.offset(), 0);

        let params = PaginationParams {
            page: 2,
            page_size: 10,
        };
        assert_eq!(params.offset(), 10);
    }

    #[test]
    fn test_validate_ingredients_ok() {
        let ingredients: Vec<CreateIngredient> = (1..=50)
            .map(|i| CreateIngredient {
                position: i,
                quantity: None,
                unit: None,
                name: format!("Ingredient {}", i),
                notes: None,
            })
            .collect();

        assert!(RecipeService::validate_ingredients(&ingredients).is_ok());
    }

    #[test]
    fn test_validate_ingredients_too_many() {
        let ingredients: Vec<CreateIngredient> = (1..=51)
            .map(|i| CreateIngredient {
                position: i,
                quantity: None,
                unit: None,
                name: format!("Ingredient {}", i),
                notes: None,
            })
            .collect();

        assert!(RecipeService::validate_ingredients(&ingredients).is_err());
    }

    #[test]
    fn test_validate_instructions_ok() {
        let steps: Vec<CreateInstructionStep> = (1..=30)
            .map(|i| CreateInstructionStep {
                step_number: i,
                description: format!("Step {}", i),
                image_url: None,
                duration_min: None,
            })
            .collect();

        assert!(RecipeService::validate_instructions(&steps).is_ok());
    }

    #[test]
    fn test_validate_instructions_too_many() {
        let steps: Vec<CreateInstructionStep> = (1..=31)
            .map(|i| CreateInstructionStep {
                step_number: i,
                description: format!("Step {}", i),
                image_url: None,
                duration_min: None,
            })
            .collect();

        assert!(RecipeService::validate_instructions(&steps).is_err());
    }
}
