//! Development seed data
//!
//! Provides test data for development and testing environments.
//! Run with `cargo run -- --seed` or `make seed`.

mod books;
mod recipes;
mod users;

use sqlx::PgPool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SeedError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Password hashing error: {0}")]
    Password(String),
}

/// Seed result containing counts of created entities
#[derive(Debug, Default)]
pub struct SeedResult {
    pub users: usize,
    pub recipes: usize,
    pub books: usize,
}

/// Run all seeds
///
/// Creates test users, sample recipes, and recipe books.
/// Safe to run multiple times - checks for existing data.
pub async fn run(pool: &PgPool) -> Result<SeedResult, SeedError> {
    // Check if data already exists
    let user_count: Option<i64> = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    let user_count = user_count.unwrap_or(0);

    if user_count > 0 {
        tracing::info!("Database already contains data, skipping seeds");
        return Ok(SeedResult::default());
    }

    tracing::info!("Seeding database with test data...");

    // Create users first (recipes depend on them)
    let user_ids = users::seed(pool).await?;
    tracing::info!("Created {} test users", user_ids.len());

    // Create recipes for each user
    let recipe_ids = recipes::seed(pool, &user_ids).await?;
    tracing::info!("Created {} sample recipes", recipe_ids.len());

    // Create recipe books
    let book_count = books::seed(pool, &user_ids, &recipe_ids).await?;
    tracing::info!("Created {} recipe books", book_count);

    Ok(SeedResult {
        users: user_ids.len(),
        recipes: recipe_ids.len(),
        books: book_count,
    })
}
