use sqlx::PgPool;
use uuid::Uuid;

use crate::lib::error::{AppError, AppResult};
use crate::lib::pagination::{PaginatedResponse, PaginationParams};
use crate::models::{
    AddRecipeToBook, BookRecipeEntry, CreateRecipeBook, RecipeBook, RecipeBookSummary,
    RecipeSummary, UpdateRecipeBook, Visibility,
};

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

    /// Get a recipe book by ID
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

    /// List books by owner
    pub async fn list_by_owner(
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

        Ok(PaginatedResponse::new(books, params.page, limit, total as u64))
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

        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM recipe_books WHERE visibility = 'public'"
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok(PaginatedResponse::new(books, params.page, limit, total as u64))
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

    /// Get recipes in a book
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
                ri.url as primary_image_url
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

        Ok(PaginatedResponse::new(recipes, params.page, limit, total as u64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_exists() {
        // Just verify the module compiles
        assert!(true);
    }
}
