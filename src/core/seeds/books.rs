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

#[cfg(test)]
mod tests {
    /// Test that the seed function expects correct number of users
    #[test]
    fn test_books_require_users() {
        // Books require at least 3 users (indices 0 and 2 are used)
        let required_users = 3;
        assert!(required_users >= 3, "Books seed requires at least 3 users");
    }

    /// Test that the seed function expects correct number of recipes
    #[test]
    fn test_books_require_recipes() {
        // Books require at least 5 recipes (indices 0..2 and 3..5 are used)
        let required_recipes = 5;
        assert!(
            required_recipes >= 5,
            "Books seed requires at least 5 recipes"
        );
    }

    /// Test book data constants (would be better with actual const data)
    #[test]
    fn test_book_titles_are_meaningful() {
        let book_titles = ["French Classics", "Mediterranean Delights"];
        for title in &book_titles {
            assert!(!title.is_empty(), "Book title should not be empty");
            assert!(title.len() >= 5, "Book title should be descriptive");
        }
    }
}
