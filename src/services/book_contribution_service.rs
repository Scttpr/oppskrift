//! Book Contribution Service for collaborative book editing
//!
//! Allows contributors to add their own recipes to books while
//! maintaining recipe ownership.

use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::models::{
    AddContributionRequest, BookContribution, BookContributionWithDisplay, PermissionLevel,
    ResourceType,
};
use crate::services::PermissionService;

/// Service for book contribution operations
pub struct BookContributionService;

impl BookContributionService {
    // =========================================================================
    // Contribution Management (T074-T078)
    // =========================================================================

    /// Add a recipe contribution to a book
    ///
    /// Validates:
    /// - User has contributor permission on the book
    /// - Recipe belongs to the user (can only contribute own recipes)
    /// - Recipe is not already in the book
    pub async fn add_contribution(
        pool: &PgPool,
        book_id: Uuid,
        contributor_id: Uuid,
        request: AddContributionRequest,
    ) -> AppResult<BookContribution> {
        // Check contributor has permission on the book
        let permission_result = PermissionService::check_permission(
            pool,
            Some(contributor_id),
            ResourceType::Book,
            book_id,
            PermissionLevel::Contributor,
        )
        .await?;

        if !permission_result.has_permission {
            return Err(AppError::NotFound("Book not found".to_string()));
        }

        // Verify recipe exists and belongs to the contributor
        let recipe = sqlx::query!(
            "SELECT id, author_id FROM recipes WHERE id = $1",
            request.recipe_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Recipe not found".to_string()))?;

        if recipe.author_id != contributor_id {
            return Err(AppError::BadRequest(
                "You can only contribute your own recipes".to_string(),
            ));
        }

        // Check if recipe is already in the book (via book_recipe_entries or contributions)
        let already_in_book = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM book_recipe_entries WHERE book_id = $1 AND recipe_id = $2
                UNION
                SELECT 1 FROM book_contributions WHERE book_id = $1 AND recipe_id = $2
            ) as "exists!"
            "#,
            book_id,
            request.recipe_id
        )
        .fetch_one(pool)
        .await?;

        if already_in_book {
            return Err(AppError::Conflict(
                "Recipe is already in this book".to_string(),
            ));
        }

        // Add the contribution with pending status
        let id = Uuid::new_v4();
        let contribution = sqlx::query_as!(
            BookContribution,
            r#"
            INSERT INTO book_contributions (id, book_id, recipe_id, contributor_id, status)
            VALUES ($1, $2, $3, $4, 'pending')
            RETURNING id, book_id, recipe_id, contributor_id, added_at, status, rejection_reason
            "#,
            id,
            book_id,
            request.recipe_id,
            contributor_id
        )
        .fetch_one(pool)
        .await?;

        Ok(contribution)
    }

    /// Remove a contribution (contributor can remove own, owner can remove any)
    pub async fn remove_contribution(
        pool: &PgPool,
        book_id: Uuid,
        recipe_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<()> {
        // Get the contribution
        let contribution = sqlx::query_as!(
            BookContribution,
            r#"
            SELECT id, book_id, recipe_id, contributor_id, added_at, status, rejection_reason
            FROM book_contributions
            WHERE book_id = $1 AND recipe_id = $2
            "#,
            book_id,
            recipe_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Contribution not found".to_string()))?;

        // Check permission: must be the contributor or the book owner
        let is_contributor = contribution.contributor_id == user_id;
        let is_owner =
            PermissionService::is_owner(pool, user_id, ResourceType::Book, book_id).await?;

        if !is_contributor && !is_owner {
            return Err(AppError::NotFound("Contribution not found".to_string()));
        }

        // Remove the contribution
        sqlx::query!(
            "DELETE FROM book_contributions WHERE id = $1",
            contribution.id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get all contributions for a book
    pub async fn get_contributions(
        pool: &PgPool,
        book_id: Uuid,
    ) -> AppResult<Vec<BookContributionWithDisplay>> {
        let contributions = sqlx::query!(
            r#"
            SELECT
                bc.id, bc.book_id, bc.recipe_id, bc.contributor_id, bc.added_at,
                bc.status, bc.rejection_reason,
                u.display_name as contributor_display_name
            FROM book_contributions bc
            JOIN users u ON u.id = bc.contributor_id
            WHERE bc.book_id = $1
            ORDER BY bc.added_at DESC
            "#,
            book_id
        )
        .fetch_all(pool)
        .await?;

        Ok(contributions
            .into_iter()
            .map(|c| BookContributionWithDisplay {
                id: c.id,
                book_id: c.book_id,
                recipe_id: c.recipe_id,
                contributor_id: c.contributor_id,
                contributor_display_name: c.contributor_display_name,
                added_at: c.added_at,
                status: c.status,
                rejection_reason: c.rejection_reason,
            })
            .collect())
    }

    /// Get pending contributions for a book (owner view)
    pub async fn get_pending_contributions(
        pool: &PgPool,
        book_id: Uuid,
    ) -> AppResult<Vec<BookContributionWithDisplay>> {
        let contributions = sqlx::query!(
            r#"
            SELECT
                bc.id, bc.book_id, bc.recipe_id, bc.contributor_id, bc.added_at,
                bc.status, bc.rejection_reason,
                u.display_name as contributor_display_name
            FROM book_contributions bc
            JOIN users u ON u.id = bc.contributor_id
            WHERE bc.book_id = $1 AND bc.status = 'pending'
            ORDER BY bc.added_at ASC
            "#,
            book_id
        )
        .fetch_all(pool)
        .await?;

        Ok(contributions
            .into_iter()
            .map(|c| BookContributionWithDisplay {
                id: c.id,
                book_id: c.book_id,
                recipe_id: c.recipe_id,
                contributor_id: c.contributor_id,
                contributor_display_name: c.contributor_display_name,
                added_at: c.added_at,
                status: c.status,
                rejection_reason: c.rejection_reason,
            })
            .collect())
    }

    /// Accept a pending contribution (T007)
    ///
    /// Only book owners can accept contributions.
    pub async fn accept_contribution(
        pool: &PgPool,
        contribution_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<BookContribution> {
        // Get the contribution
        let contribution = sqlx::query_as!(
            BookContribution,
            r#"
            SELECT id, book_id, recipe_id, contributor_id, added_at, status, rejection_reason
            FROM book_contributions
            WHERE id = $1
            "#,
            contribution_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Contribution not found".to_string()))?;

        // Check user is book owner
        let is_owner =
            PermissionService::is_owner(pool, user_id, ResourceType::Book, contribution.book_id)
                .await?;

        if !is_owner {
            return Err(AppError::NotFound("Contribution not found".to_string()));
        }

        // Must be pending to accept
        if contribution.status != "pending" {
            return Err(AppError::BadRequest(
                "Only pending contributions can be accepted".to_string(),
            ));
        }

        // Update status to accepted
        let updated = sqlx::query_as!(
            BookContribution,
            r#"
            UPDATE book_contributions
            SET status = 'accepted', rejection_reason = NULL
            WHERE id = $1
            RETURNING id, book_id, recipe_id, contributor_id, added_at, status, rejection_reason
            "#,
            contribution_id
        )
        .fetch_one(pool)
        .await?;

        Ok(updated)
    }

    /// Reject a pending contribution (T007)
    ///
    /// Only book owners can reject contributions.
    /// An optional reason can be provided.
    pub async fn reject_contribution(
        pool: &PgPool,
        contribution_id: Uuid,
        user_id: Uuid,
        reason: Option<String>,
    ) -> AppResult<BookContribution> {
        // Get the contribution
        let contribution = sqlx::query_as!(
            BookContribution,
            r#"
            SELECT id, book_id, recipe_id, contributor_id, added_at, status, rejection_reason
            FROM book_contributions
            WHERE id = $1
            "#,
            contribution_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Contribution not found".to_string()))?;

        // Check user is book owner
        let is_owner =
            PermissionService::is_owner(pool, user_id, ResourceType::Book, contribution.book_id)
                .await?;

        if !is_owner {
            return Err(AppError::NotFound("Contribution not found".to_string()));
        }

        // Must be pending to reject
        if contribution.status != "pending" {
            return Err(AppError::BadRequest(
                "Only pending contributions can be rejected".to_string(),
            ));
        }

        // Validate reason length if provided
        let sanitized_reason = reason.map(|r| {
            // Truncate to 500 chars and sanitize
            let truncated = if r.len() > 500 { &r[..500] } else { &r };
            truncated.trim().to_string()
        });

        // Update status to rejected with reason
        let updated = sqlx::query_as!(
            BookContribution,
            r#"
            UPDATE book_contributions
            SET status = 'rejected', rejection_reason = $2
            WHERE id = $1
            RETURNING id, book_id, recipe_id, contributor_id, added_at, status, rejection_reason
            "#,
            contribution_id,
            sanitized_reason
        )
        .fetch_one(pool)
        .await?;

        Ok(updated)
    }

    /// Get contribution count for a book (only accepted)
    pub async fn get_contribution_count(pool: &PgPool, book_id: Uuid) -> AppResult<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM book_contributions WHERE book_id = $1 AND status = 'accepted'",
            book_id
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok(count)
    }

    /// Get pending contribution count for a book
    pub async fn get_pending_count(pool: &PgPool, book_id: Uuid) -> AppResult<i64> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM book_contributions WHERE book_id = $1 AND status = 'pending'",
            book_id
        )
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_service_exists() {
        // Basic sanity test
    }
}
