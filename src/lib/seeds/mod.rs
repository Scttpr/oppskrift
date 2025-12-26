//! Seeds module - Development seed data (users.rs, recipes.rs)
//!
//! Use `cargo run -- --seed` to run seeds in development.

#![allow(dead_code)]

pub mod recipes;
pub mod users;

use sqlx::PgPool;

use crate::lib::error::AppResult;

/// Environment detection for seeding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Environment {
    /// Detect current environment from RUST_ENV or APP_ENV
    pub fn detect() -> Self {
        let env = std::env::var("RUST_ENV")
            .or_else(|_| std::env::var("APP_ENV"))
            .unwrap_or_else(|_| "development".to_string());

        match env.to_lowercase().as_str() {
            "production" | "prod" => Self::Production,
            "test" | "testing" => Self::Test,
            _ => Self::Development,
        }
    }

    /// Check if seeding is allowed
    pub fn allows_seeding(&self) -> bool {
        matches!(self, Self::Development | Self::Test)
    }
}

/// Run all seeds
pub async fn run_seeds(pool: &PgPool, base_url: &str) -> AppResult<()> {
    let env = Environment::detect();

    if !env.allows_seeding() {
        tracing::warn!("Seeding is not allowed in {:?} environment", env);
        return Ok(());
    }

    tracing::info!("Running seeds in {:?} environment", env);

    // Seed users first (recipes depend on them)
    let user_ids = users::seed_users(pool, base_url).await?;
    tracing::info!("Seeded {} users", user_ids.len());

    // Seed recipes
    let recipe_count = recipes::seed_recipes(pool, &user_ids, base_url).await?;
    tracing::info!("Seeded {} recipes", recipe_count);

    tracing::info!("Seeding complete!");
    Ok(())
}

/// Check if seeds should be run based on CLI args
pub fn should_seed() -> bool {
    std::env::args().any(|arg| arg == "--seed")
}
