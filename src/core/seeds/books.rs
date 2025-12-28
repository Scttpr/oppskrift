//! Recipe book seed data

use sqlx::PgPool;
use uuid::Uuid;

use super::SeedError;

/// Seed recipe books
///
/// Creates 2 recipe books with recipes assigned.
pub async fn seed(
    pool: &PgPool,
    user_ids: &[Uuid],
    recipe_ids: &[Uuid],
) -> Result<usize, SeedError> {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Book 1: Alice's French Favorites (contains omelette and tarte tatin)
    let book1_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO recipe_books (owner_id, title, description, visibility, ap_id)
        VALUES ($1, $2, $3, 'public'::visibility_type, $4)
        RETURNING id
        "#,
    )
    .bind(user_ids[0]) // alice
    .bind("French Classics")
    .bind("My collection of timeless French recipes, from simple omelettes to elegant desserts.")
    .bind(format!("{}/books/french-classics", base_url))
    .fetch_one(pool)
    .await?;

    // Add recipes to book 1 (omelette and tarte tatin)
    for (position, &recipe_id) in recipe_ids[0..2].iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO book_recipe_entries (book_id, recipe_id, position)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(book1_id)
        .bind(recipe_id)
        .bind((position + 1) as i32)
        .execute(pool)
        .await?;
    }

    // Book 2: Chef Marie's Mediterranean Collection
    let book2_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO recipe_books (owner_id, title, description, visibility, ap_id)
        VALUES ($1, $2, $3, 'public'::visibility_type, $4)
        RETURNING id
        "#,
    )
    .bind(user_ids[2]) // chef_marie
    .bind("Mediterranean Delights")
    .bind("Fresh, healthy recipes inspired by the Mediterranean coast. Focus on quality ingredients and simple techniques.")
    .bind(format!("{}/books/mediterranean-delights", base_url))
    .fetch_one(pool)
    .await?;

    // Add recipes to book 2 (sea bass and chocolate fondant)
    for (position, &recipe_id) in recipe_ids[3..5].iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO book_recipe_entries (book_id, recipe_id, position)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(book2_id)
        .bind(recipe_id)
        .bind((position + 1) as i32)
        .execute(pool)
        .await?;
    }

    Ok(2)
}
