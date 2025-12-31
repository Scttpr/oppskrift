use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::core::pagination::{PaginatedResponse, PaginationParams};
use crate::models::{
    AddRecipeToBook, BookRecipeEntry, CreateRecipeBook, PermissionLevel, RecipeBook,
    RecipeBookSummary, RecipeSummary, ResourceType, UpdateRecipeBook, Visibility,
};
use crate::services::PermissionService;

/// Service for recipe book operations
pub struct BookService;

impl BookService {
    /// Create a new recipe book
    pub async fn create(
        pool: &PgPool,
        owner_id: Uuid,
        input: CreateRecipeBook,
        base_url: &str,
    ) -> AppResult<RecipeBook> {
        let id = Uuid::new_v4();
        let ap_id = format!("{}/books/{}", base_url, id);
        let visibility = input.visibility.unwrap_or_default();

        sqlx::query_as!(
            RecipeBook,
            r#"
            INSERT INTO recipe_books (id, owner_id, title, description, cover_image_url, visibility, ap_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id, owner_id, title, description, cover_image_url,
                visibility as "visibility: Visibility",
                created_at, updated_at, ap_id
            "#,
            id,
            owner_id,
            input.title,
            input.description,
            input.cover_image_url,
            visibility as Visibility,
            ap_id
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::from)
    }

    /// Get a recipe book by ID (internal, no visibility check)
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> AppResult<RecipeBook> {
        sqlx::query_as!(
            RecipeBook,
            r#"
            SELECT
                id, owner_id, title, description, cover_image_url,
                visibility as "visibility: Visibility",
                created_at, updated_at, ap_id
            FROM recipe_books
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Recipe book {} not found", id)))
    }

    /// Get a recipe book by ID with permission check
    /// Uses PermissionService to check access (owner, direct share, group, followers, public)
    /// Returns 404 for unauthorized access to hide resource existence
    pub async fn get_by_id_authorized(
        pool: &PgPool,
        id: Uuid,
        viewer_id: Option<Uuid>,
    ) -> AppResult<RecipeBook> {
        // Use PermissionService to check view access
        PermissionService::require_permission(
            pool,
            viewer_id,
            ResourceType::Book,
            id,
            PermissionLevel::View,
        )
        .await?;

        // Permission check passed, fetch the full book
        Self::get_by_id(pool, id).await
    }

    /// Check if user has edit permission on a book
    /// Returns 404 for unauthorized access
    pub async fn require_edit_permission(
        pool: &PgPool,
        book_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<()> {
        PermissionService::require_permission(
            pool,
            Some(user_id),
            ResourceType::Book,
            book_id,
            PermissionLevel::Edit,
        )
        .await?;
        Ok(())
    }

    /// Update a recipe book
    pub async fn update(pool: &PgPool, id: Uuid, input: UpdateRecipeBook) -> AppResult<RecipeBook> {
        sqlx::query_as!(
            RecipeBook,
            r#"
            UPDATE recipe_books
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                cover_image_url = COALESCE($4, cover_image_url),
                visibility = COALESCE($5, visibility),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, owner_id, title, description, cover_image_url,
                visibility as "visibility: Visibility",
                created_at, updated_at, ap_id
            "#,
            id,
            input.title,
            input.description,
            input.cover_image_url,
            input.visibility as Option<Visibility>
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Recipe book {} not found", id)))
    }

    /// Delete a recipe book
    pub async fn delete(pool: &PgPool, id: Uuid) -> AppResult<()> {
        let result = sqlx::query!("DELETE FROM recipe_books WHERE id = $1", id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Recipe book {} not found", id)));
        }

        Ok(())
    }

    /// List all books by owner (for dropdowns, no pagination)
    pub async fn list_by_owner(pool: &PgPool, owner_id: Uuid) -> AppResult<Vec<RecipeBookSummary>> {
        let books = sqlx::query_as!(
            RecipeBookSummary,
            r#"
            SELECT
                b.id, b.owner_id, b.title, b.description, b.cover_image_url,
                b.visibility as "visibility: Visibility",
                b.created_at,
                COUNT(e.id) as "recipe_count!"
            FROM recipe_books b
            LEFT JOIN book_recipe_entries e ON e.book_id = b.id
            WHERE b.owner_id = $1
            GROUP BY b.id
            ORDER BY b.title
            "#,
            owner_id
        )
        .fetch_all(pool)
        .await?;

        Ok(books)
    }

    /// List books by owner with pagination
    pub async fn list_by_owner_paginated(
        pool: &PgPool,
        owner_id: Uuid,
        params: &PaginationParams,
    ) -> AppResult<PaginatedResponse<RecipeBookSummary>> {
        let limit = params.limit();
        let offset = params.offset();

        let books = sqlx::query_as!(
            RecipeBookSummary,
            r#"
            SELECT
                b.id, b.owner_id, b.title, b.description, b.cover_image_url,
                b.visibility as "visibility: Visibility",
                b.created_at,
                COUNT(e.id) as "recipe_count!"
            FROM recipe_books b
            LEFT JOIN book_recipe_entries e ON e.book_id = b.id
            WHERE b.owner_id = $1
            GROUP BY b.id
            ORDER BY b.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            owner_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM recipe_books WHERE owner_id = $1",
            owner_id
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok(PaginatedResponse::new(
            books,
            params.page,
            limit,
            total as u64,
        ))
    }

    /// List public books
    pub async fn list_public(
        pool: &PgPool,
        params: &PaginationParams,
    ) -> AppResult<PaginatedResponse<RecipeBookSummary>> {
        let limit = params.limit();
        let offset = params.offset();

        let books = sqlx::query_as!(
            RecipeBookSummary,
            r#"
            SELECT
                b.id, b.owner_id, b.title, b.description, b.cover_image_url,
                b.visibility as "visibility: Visibility",
                b.created_at,
                COUNT(e.id) as "recipe_count!"
            FROM recipe_books b
            LEFT JOIN book_recipe_entries e ON e.book_id = b.id
            WHERE b.visibility = 'public'
            GROUP BY b.id
            ORDER BY b.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?;

        let total: i64 =
            sqlx::query_scalar!("SELECT COUNT(*) FROM recipe_books WHERE visibility = 'public'")
                .fetch_one(pool)
                .await?
                .unwrap_or(0);

        Ok(PaginatedResponse::new(
            books,
            params.page,
            limit,
            total as u64,
        ))
    }

    /// Add a recipe to a book
    pub async fn add_recipe(
        pool: &PgPool,
        book_id: Uuid,
        input: AddRecipeToBook,
    ) -> AppResult<BookRecipeEntry> {
        // Get next position if not specified
        let position = match input.position {
            Some(p) => p,
            None => {
                let max: Option<i32> = sqlx::query_scalar!(
                    "SELECT MAX(position) FROM book_recipe_entries WHERE book_id = $1",
                    book_id
                )
                .fetch_one(pool)
                .await?;
                max.unwrap_or(0) + 1
            }
        };

        sqlx::query_as!(
            BookRecipeEntry,
            r#"
            INSERT INTO book_recipe_entries (book_id, recipe_id, position)
            VALUES ($1, $2, $3)
            RETURNING id, book_id, recipe_id, position, added_at
            "#,
            book_id,
            input.recipe_id,
            position
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err)
                if db_err.constraint() == Some("book_recipe_entries_unique") =>
            {
                AppError::Conflict("Recipe is already in this book".to_string())
            }
            _ => AppError::from(e),
        })
    }

    /// Remove a recipe from a book
    pub async fn remove_recipe(pool: &PgPool, book_id: Uuid, recipe_id: Uuid) -> AppResult<()> {
        let result = sqlx::query!(
            "DELETE FROM book_recipe_entries WHERE book_id = $1 AND recipe_id = $2",
            book_id,
            recipe_id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Recipe not found in this book".to_string(),
            ));
        }

        Ok(())
    }

    /// Get book IDs that contain a specific recipe (for a specific owner)
    pub async fn get_book_ids_containing_recipe(
        pool: &PgPool,
        owner_id: Uuid,
        recipe_id: Uuid,
    ) -> AppResult<Vec<Uuid>> {
        let book_ids = sqlx::query_scalar!(
            r#"
            SELECT b.id
            FROM recipe_books b
            INNER JOIN book_recipe_entries e ON e.book_id = b.id
            WHERE b.owner_id = $1 AND e.recipe_id = $2
            "#,
            owner_id,
            recipe_id
        )
        .fetch_all(pool)
        .await?;

        Ok(book_ids)
    }

    /// Get recipes in a book (all recipes, for owner)
    pub async fn get_recipes_in_book(
        pool: &PgPool,
        book_id: Uuid,
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
                r.difficulty as "difficulty: crate::models::Difficulty",
                r.created_at,
                ri.url as "primary_image_url?"
            FROM recipes r
            INNER JOIN book_recipe_entries e ON e.recipe_id = r.id
            LEFT JOIN recipe_images ri ON ri.recipe_id = r.id AND ri.is_primary = true
            WHERE e.book_id = $1
            ORDER BY e.position
            LIMIT $2 OFFSET $3
            "#,
            book_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM book_recipe_entries WHERE book_id = $1",
            book_id
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

    /// Get public recipes in a book (for non-owners, filters out private recipes)
    pub async fn get_public_recipes_in_book(
        pool: &PgPool,
        book_id: Uuid,
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
                r.difficulty as "difficulty: crate::models::Difficulty",
                r.created_at,
                ri.url as "primary_image_url?"
            FROM recipes r
            INNER JOIN book_recipe_entries e ON e.recipe_id = r.id
            LEFT JOIN recipe_images ri ON ri.recipe_id = r.id AND ri.is_primary = true
            WHERE e.book_id = $1 AND r.visibility = 'public'
            ORDER BY e.position
            LIMIT $2 OFFSET $3
            "#,
            book_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) FROM book_recipe_entries e
            INNER JOIN recipes r ON r.id = e.recipe_id
            WHERE e.book_id = $1 AND r.visibility = 'public'
            "#,
            book_id
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
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Visibility Tests (T042 - Error Paths)
    // ==========================================================================

    #[test]
    fn test_visibility_default_is_private() {
        // Default is Private for privacy-first design (ABAC spec)
        let default = Visibility::default();
        assert_eq!(default, Visibility::Private);
    }

    #[test]
    fn test_visibility_variants() {
        assert_eq!(Visibility::Public.to_string(), "Public");
        assert_eq!(Visibility::Private.to_string(), "Private");
    }

    // ==========================================================================
    // Pagination Parameter Tests (T042)
    // ==========================================================================

    #[test]
    fn test_pagination_params_limit() {
        let params = PaginationParams {
            page: 1,
            page_size: 20,
        };
        assert_eq!(params.limit(), 20);
    }

    #[test]
    fn test_pagination_params_offset() {
        let params = PaginationParams {
            page: 3,
            page_size: 10,
        };
        assert_eq!(params.offset(), 20); // (3-1) * 10
    }

    #[test]
    fn test_pagination_params_first_page() {
        let params = PaginationParams {
            page: 1,
            page_size: 25,
        };
        assert_eq!(params.offset(), 0);
        assert_eq!(params.limit(), 25);
    }

    // ==========================================================================
    // AddRecipeToBook Input Tests (T042)
    // ==========================================================================

    #[test]
    fn test_add_recipe_to_book_with_position() {
        let input = AddRecipeToBook {
            recipe_id: Uuid::new_v4(),
            position: Some(5),
        };
        assert_eq!(input.position, Some(5));
    }

    #[test]
    fn test_add_recipe_to_book_without_position() {
        let input = AddRecipeToBook {
            recipe_id: Uuid::new_v4(),
            position: None,
        };
        assert!(input.position.is_none());
    }

    // ==========================================================================
    // CreateRecipeBook Input Tests (T042)
    // ==========================================================================

    #[test]
    fn test_create_recipe_book_minimal() {
        let input = CreateRecipeBook {
            title: "My Favorites".to_string(),
            description: None,
            cover_image_url: None,
            visibility: None,
        };
        assert_eq!(input.title, "My Favorites");
        assert!(input.visibility.is_none());
    }

    #[test]
    fn test_create_recipe_book_full() {
        let input = CreateRecipeBook {
            title: "Holiday Recipes".to_string(),
            description: Some("Best recipes for the holidays".to_string()),
            cover_image_url: Some("https://example.com/cover.jpg".to_string()),
            visibility: Some(Visibility::Public),
        };
        assert_eq!(input.visibility, Some(Visibility::Public));
    }

    // ==========================================================================
    // UpdateRecipeBook Input Tests (T042)
    // ==========================================================================

    #[test]
    fn test_update_recipe_book_partial() {
        let input = UpdateRecipeBook {
            title: Some("Updated Title".to_string()),
            description: None,
            cover_image_url: None,
            visibility: None,
        };
        assert_eq!(input.title, Some("Updated Title".to_string()));
        assert!(input.description.is_none());
    }

    #[test]
    fn test_update_recipe_book_visibility_only() {
        let input = UpdateRecipeBook {
            title: None,
            description: None,
            cover_image_url: None,
            visibility: Some(Visibility::Public),
        };
        assert_eq!(input.visibility, Some(Visibility::Public));
    }

    // ==========================================================================
    // UUID Tests (T042)
    // ==========================================================================

    #[test]
    fn test_uuid_generation() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        assert_ne!(id1, id2, "UUIDs should be unique");
    }

    #[test]
    fn test_ap_id_format() {
        let base_url = "https://oppskrift.example.com";
        let id = Uuid::new_v4();
        let ap_id = format!("{}/books/{}", base_url, id);
        assert!(ap_id.starts_with(base_url));
        assert!(ap_id.contains("/books/"));
    }
}
