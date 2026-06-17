use sqlx::PgPool;
use uuid::Uuid;

use crate::core::audit::AuditEvent;
use crate::core::error::{AppError, AppResult};
use crate::models::{Comment, CommentWithAuthor, MAX_COMMENT_LEN};

/// Service for recipe comments
pub struct CommentService;

impl CommentService {
    /// Add a comment to a recipe. The body is trimmed and length-validated.
    pub async fn add_comment(
        pool: &PgPool,
        recipe_id: Uuid,
        author_id: Uuid,
        body: &str,
    ) -> AppResult<Comment> {
        let body = body.trim();
        if body.is_empty() {
            return Err(AppError::Validation("Comment cannot be empty".to_string()));
        }
        if body.chars().count() > MAX_COMMENT_LEN {
            return Err(AppError::Validation(format!(
                "Comment cannot exceed {} characters",
                MAX_COMMENT_LEN
            )));
        }

        let comment = sqlx::query_as!(
            Comment,
            r#"
            INSERT INTO recipe_comments (recipe_id, author_id, body)
            VALUES ($1, $2, $3)
            RETURNING id, recipe_id, author_id, body, created_at, updated_at
            "#,
            recipe_id,
            author_id,
            body
        )
        .fetch_one(pool)
        .await?;

        AuditEvent::new("comment.create")
            .with_user(author_id)
            .with_target("recipe", recipe_id)
            .log();

        Ok(comment)
    }

    /// List comments for a recipe with author display names, newest first.
    pub async fn list_comments(
        pool: &PgPool,
        recipe_id: Uuid,
    ) -> AppResult<Vec<CommentWithAuthor>> {
        let comments = sqlx::query_as!(
            CommentWithAuthor,
            r#"
            SELECT
                c.id, c.recipe_id, c.author_id,
                COALESCE(NULLIF(u.display_name, ''), u.username) as "author_name!",
                c.body, c.created_at
            FROM recipe_comments c
            JOIN users u ON u.id = c.author_id
            WHERE c.recipe_id = $1
            ORDER BY c.created_at DESC
            "#,
            recipe_id
        )
        .fetch_all(pool)
        .await?;

        Ok(comments)
    }

    /// Delete a comment. Allowed for the comment's author or the recipe's owner.
    /// Returns 404 if the comment does not exist on the given recipe.
    pub async fn delete_comment(
        pool: &PgPool,
        recipe_id: Uuid,
        comment_id: Uuid,
        actor_id: Uuid,
    ) -> AppResult<()> {
        let row = sqlx::query!(
            r#"
            SELECT c.author_id, r.author_id as recipe_owner
            FROM recipe_comments c
            JOIN recipes r ON r.id = c.recipe_id
            WHERE c.id = $1 AND c.recipe_id = $2
            "#,
            comment_id,
            recipe_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Comment {} not found", comment_id)))?;

        if row.author_id != actor_id && row.recipe_owner != actor_id {
            // Hide existence from users who can neither author- nor owner-delete.
            return Err(AppError::NotFound(format!(
                "Comment {} not found",
                comment_id
            )));
        }

        sqlx::query!("DELETE FROM recipe_comments WHERE id = $1", comment_id)
            .execute(pool)
            .await?;

        AuditEvent::new("comment.delete")
            .with_user(actor_id)
            .with_target("recipe", recipe_id)
            .log();

        Ok(())
    }
}
