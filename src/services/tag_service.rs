use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::core::pagination::{paginate, PaginatedResponse, PaginationParams};
use crate::models::{slugify, Difficulty, RecipeSummary, Tag, TagWithCount, MAX_TAGS_PER_RECIPE};

/// Service for tag / category operations
pub struct TagService;

impl TagService {
    /// Replace the full set of tags on a recipe.
    ///
    /// Names are normalized to slugs, de-duplicated, and capped at
    /// [`MAX_TAGS_PER_RECIPE`]. Unknown tags are created on the fly. Runs
    /// inside the caller's transaction so it is atomic with the recipe write.
    pub async fn set_recipe_tags(
        conn: &mut sqlx::PgConnection,
        recipe_id: Uuid,
        names: &[String],
    ) -> AppResult<()> {
        // Normalize to (slug, display name), de-duplicated by slug, order-preserving.
        let mut seen = std::collections::HashSet::new();
        let mut normalized: Vec<(String, String)> = Vec::new();
        for raw in names {
            if let Some(slug) = slugify(raw) {
                if seen.insert(slug.clone()) {
                    normalized.push((slug, raw.trim().to_string()));
                }
            }
        }

        if normalized.len() > MAX_TAGS_PER_RECIPE {
            return Err(AppError::Validation(format!(
                "Recipe cannot have more than {} tags (got {})",
                MAX_TAGS_PER_RECIPE,
                normalized.len()
            )));
        }

        // Clear existing associations, then re-link.
        sqlx::query!("DELETE FROM recipe_tags WHERE recipe_id = $1", recipe_id)
            .execute(&mut *conn)
            .await?;

        for (slug, name) in normalized {
            let tag_id: Uuid = sqlx::query_scalar!(
                r#"
                INSERT INTO tags (name, slug)
                VALUES ($1, $2)
                ON CONFLICT (slug) DO UPDATE SET slug = EXCLUDED.slug
                RETURNING id
                "#,
                name,
                slug
            )
            .fetch_one(&mut *conn)
            .await?;

            sqlx::query!(
                r#"
                INSERT INTO recipe_tags (recipe_id, tag_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
                recipe_id,
                tag_id
            )
            .execute(&mut *conn)
            .await?;
        }

        Ok(())
    }

    /// Get all tags attached to a recipe, ordered by name.
    pub async fn get_recipe_tags(pool: &PgPool, recipe_id: Uuid) -> AppResult<Vec<Tag>> {
        let tags = sqlx::query_as!(
            Tag,
            r#"
            SELECT t.id, t.name, t.slug, t.created_at
            FROM tags t
            JOIN recipe_tags rt ON rt.tag_id = t.id
            WHERE rt.recipe_id = $1
            ORDER BY t.name
            "#,
            recipe_id
        )
        .fetch_all(pool)
        .await?;

        Ok(tags)
    }

    /// List all tags that have at least one public recipe, with counts.
    pub async fn list_with_counts(pool: &PgPool) -> AppResult<Vec<TagWithCount>> {
        let tags = sqlx::query_as!(
            TagWithCount,
            r#"
            SELECT
                t.id, t.name, t.slug,
                COUNT(r.id) as "recipe_count!"
            FROM tags t
            JOIN recipe_tags rt ON rt.tag_id = t.id
            JOIN recipes r ON r.id = rt.recipe_id AND r.visibility = 'public'
            GROUP BY t.id, t.name, t.slug
            ORDER BY COUNT(r.id) DESC, t.name
            "#
        )
        .fetch_all(pool)
        .await?;

        Ok(tags)
    }

    /// Look up a tag by its slug.
    pub async fn get_by_slug(pool: &PgPool, slug: &str) -> AppResult<Tag> {
        sqlx::query_as!(
            Tag,
            r#"
            SELECT id, name, slug, created_at
            FROM tags
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Tag {} not found", slug)))
    }

    /// List public recipes carrying a given tag (by slug), newest first.
    pub async fn list_public_recipes_by_tag(
        pool: &PgPool,
        slug: &str,
        params: &PaginationParams,
    ) -> AppResult<PaginatedResponse<RecipeSummary>> {
        paginate(
            params,
            |limit, offset| {
                sqlx::query_as!(
                    RecipeSummary,
                    r#"
                    SELECT
                        r.id, r.author_id, r.title, r.description,
                        r.prep_time_min, r.cook_time_min,
                        r.difficulty as "difficulty: Difficulty",
                        r.created_at,
                        ri.url as "primary_image_url?"
                    FROM recipes r
                    JOIN recipe_tags rt ON rt.recipe_id = r.id
                    JOIN tags t ON t.id = rt.tag_id
                    LEFT JOIN recipe_images ri ON ri.recipe_id = r.id AND ri.is_primary = true
                    WHERE r.visibility = 'public' AND t.slug = $1
                    ORDER BY r.created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                    slug,
                    limit,
                    offset
                )
                .fetch_all(pool)
            },
            || {
                sqlx::query_scalar!(
                    r#"
                    SELECT COUNT(*) as "count!"
                    FROM recipes r
                    JOIN recipe_tags rt ON rt.recipe_id = r.id
                    JOIN tags t ON t.id = rt.tag_id
                    WHERE r.visibility = 'public' AND t.slug = $1
                    "#,
                    slug
                )
                .fetch_one(pool)
            },
        )
        .await
    }
}
