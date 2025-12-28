//! Shared test utilities for integration tests
//!
//! Uses axum-test to test against the app directly without HTTP server.

pub mod assertions;
pub mod fixtures;
pub mod security;

use axum_test::TestServer;
use chrono::{Duration, Utc};
use oppskrift::{test_app_router, AppState};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

/// Test context with database connection and test server
pub struct TestContext {
    pub db: PgPool,
    pub server: TestServer,
    created_users: Vec<Uuid>,
    created_recipes: Vec<Uuid>,
    created_books: Vec<Uuid>,
}

impl TestContext {
    /// Create a new test context with embedded test server
    pub async fn new() -> Self {
        dotenvy::dotenv().ok();

        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");

        let db = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        // Create app state and router (using test_app_router for mock ConnectInfo)
        let state = AppState { db: db.clone() };
        let app = test_app_router(state);

        // Create test server
        let server = TestServer::new(app).expect("Failed to create test server");

        Self {
            db,
            server,
            created_users: Vec::new(),
            created_recipes: Vec::new(),
            created_books: Vec::new(),
        }
    }

    /// Generate a unique test email
    pub fn unique_email() -> String {
        format!("test_{}@example.com", Uuid::new_v4())
    }

    /// Generate a unique test username
    pub fn unique_username() -> String {
        let id = Uuid::new_v4().to_string().replace('-', "");
        format!("test_{}", &id[..12])
    }

    /// Create a test user directly in database (bypassing API)
    pub async fn create_user(
        &mut self,
        email: &str,
        username: &str,
        password: &str,
        verified: bool,
    ) -> Uuid {
        use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
        use rand::rngs::OsRng;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .expect("Failed to hash password")
            .to_string();

        let ap_id = format!("http://localhost/users/{}", username);

        let user_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO users (username, email, email_verified, password_hash, display_name, ap_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id
            "#,
        )
        .bind(username)
        .bind(email)
        .bind(verified)
        .bind(&password_hash)
        .bind(username) // display_name = username
        .bind(&ap_id)
        .fetch_one(&self.db)
        .await
        .expect("Failed to create test user");

        self.created_users.push(user_id);
        user_id
    }

    /// Track a user for cleanup
    pub fn track_user(&mut self, user_id: Uuid) {
        self.created_users.push(user_id);
    }

    // ==========================================================================
    // Recipe Helpers (T005)
    // ==========================================================================

    /// Create a test recipe directly in database
    pub async fn create_recipe(&mut self, user_id: Uuid, title: &str, visibility: &str) -> Uuid {
        let ap_id = format!("http://localhost/recipes/{}", Uuid::new_v4());
        let recipe_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO recipes (author_id, title, description, visibility, prep_time_min, cook_time_min, servings, difficulty, ap_id)
            VALUES ($1, $2, $3, $4::visibility_type, 15, 30, '4', 'medium', $5)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(title)
        .bind(format!("Test description for {}", title))
        .bind(visibility)
        .bind(&ap_id)
        .fetch_one(&self.db)
        .await
        .expect("Failed to create test recipe");

        self.created_recipes.push(recipe_id);
        recipe_id
    }

    /// Create test ingredients for a recipe
    pub async fn create_ingredients(&self, recipe_id: Uuid, count: usize) {
        for i in 1..=count {
            sqlx::query(
                r#"
                INSERT INTO ingredients (recipe_id, name, quantity, unit, position)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(recipe_id)
            .bind(format!("Ingredient {}", i))
            .bind(i as f64)
            .bind("units")
            .bind(i as i32)
            .execute(&self.db)
            .await
            .expect("Failed to create ingredient");
        }
    }

    /// Create test instruction steps for a recipe
    pub async fn create_instructions(&self, recipe_id: Uuid, count: usize) {
        for i in 1..=count {
            sqlx::query(
                r#"
                INSERT INTO instruction_steps (recipe_id, step_number, description)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(recipe_id)
            .bind(i as i32)
            .bind(format!("Step {} instructions", i))
            .execute(&self.db)
            .await
            .expect("Failed to create instruction step");
        }
    }

    /// Create a complete test recipe with ingredients and instructions
    pub async fn create_complete_recipe(
        &mut self,
        user_id: Uuid,
        title: &str,
        visibility: &str,
    ) -> Uuid {
        let recipe_id = self.create_recipe(user_id, title, visibility).await;
        self.create_ingredients(recipe_id, 3).await;
        self.create_instructions(recipe_id, 3).await;
        recipe_id
    }

    // ==========================================================================
    // Book Helpers (T006)
    // ==========================================================================

    /// Create a test recipe book directly in database
    pub async fn create_book(&mut self, user_id: Uuid, name: &str, visibility: &str) -> Uuid {
        let ap_id = format!("http://localhost/books/{}", Uuid::new_v4());
        let book_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO recipe_books (owner_id, title, description, visibility, ap_id)
            VALUES ($1, $2, $3, $4::visibility_type, $5)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(name)
        .bind(format!("Description for {}", name))
        .bind(visibility)
        .bind(&ap_id)
        .fetch_one(&self.db)
        .await
        .expect("Failed to create test book");

        self.created_books.push(book_id);
        book_id
    }

    /// Add a recipe to a book
    pub async fn add_recipe_to_book(&self, book_id: Uuid, recipe_id: Uuid) {
        // Get the next position in the book
        let next_position: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM book_recipe_entries WHERE book_id = $1",
        )
        .bind(book_id)
        .fetch_one(&self.db)
        .await
        .unwrap_or(1);

        sqlx::query(
            r#"
            INSERT INTO book_recipe_entries (book_id, recipe_id, position)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(book_id)
        .bind(recipe_id)
        .bind(next_position as i32)
        .execute(&self.db)
        .await
        .expect("Failed to add recipe to book");
    }

    // ==========================================================================
    // Social Helpers (T007)
    // ==========================================================================

    /// Create a follow relationship between users
    pub async fn create_follow(&self, follower_id: Uuid, following_id: Uuid) {
        let ap_id = format!("http://localhost/follows/{}", Uuid::new_v4());
        sqlx::query(
            r#"
            INSERT INTO follows (follower_id, following_id, ap_id)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(follower_id)
        .bind(following_id)
        .bind(&ap_id)
        .execute(&self.db)
        .await
        .expect("Failed to create follow");
    }

    /// Create a test activity
    pub async fn create_activity(
        &self,
        user_id: Uuid,
        activity_type: &str,
        target_type: &str,
        target_id: Uuid,
    ) -> Uuid {
        let activity_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO activities (user_id, activity_type, target_type, target_id)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(activity_type)
        .bind(target_type)
        .bind(target_id)
        .fetch_one(&self.db)
        .await
        .expect("Failed to create activity");

        activity_id
    }

    // ==========================================================================
    // Authentication Helpers (T009)
    // ==========================================================================

    /// Login a user and return the session cookie
    pub async fn login_and_get_session(&self, email: &str, password: &str) -> Option<String> {
        let response = self
            .post(
                "/api/v1/auth/login",
                serde_json::json!({
                    "email": email,
                    "password": password
                }),
            )
            .await;

        response.session_cookie
    }

    /// Create a verified user and login, returning session
    pub async fn create_and_login(&mut self, suffix: &str) -> (Uuid, String) {
        let email = Self::unique_email();
        let username = Self::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        let user_id = self.create_user(&email, &username, password, true).await;
        let session = self
            .login_and_get_session(&email, password)
            .await
            .unwrap_or_else(|| panic!("Failed to login user {}", suffix));

        (user_id, session)
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Option<Uuid> {
        sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.db)
            .await
            .expect("Failed to query user")
    }

    /// Check if email confirmation token exists
    #[allow(dead_code)]
    pub async fn get_confirmation_token(&self, user_id: Uuid) -> Option<String> {
        sqlx::query_scalar(
            "SELECT token_hash FROM email_confirmation_tokens WHERE user_id = $1 AND expires_at > NOW()",
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .expect("Failed to query confirmation token")
    }

    /// Create an email confirmation token for testing
    /// Returns the raw token (not hashed) to use in API calls
    pub async fn create_email_confirmation_token(
        &self,
        user_id: Uuid,
        email: &str,
        expired: bool,
    ) -> String {
        // Generate random bytes like the server does
        let token_bytes: [u8; 32] = rand::random();
        let token = hex::encode(token_bytes);

        // Hash the raw bytes (not the hex string) like the server does
        let mut hasher = Sha256::new();
        hasher.update(token_bytes);
        let token_hash = hex::encode(hasher.finalize());

        // Set expiry (24 hours normally, or in the past if expired)
        let expires_at = if expired {
            Utc::now() - Duration::hours(1)
        } else {
            Utc::now() + Duration::hours(24)
        };

        sqlx::query(
            r#"
            INSERT INTO email_confirmation_tokens (user_id, email, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(user_id)
        .bind(email)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.db)
        .await
        .expect("Failed to create email confirmation token");

        token
    }

    /// Create a password reset token for testing
    /// Returns the raw token (not hashed) to use in API calls
    pub async fn create_password_reset_token(&self, user_id: Uuid, expired: bool) -> String {
        // Generate random bytes like the server does
        let token_bytes: [u8; 32] = rand::random();
        let token = hex::encode(token_bytes);

        // Hash the raw bytes (not the hex string) like the server does
        let mut hasher = Sha256::new();
        hasher.update(token_bytes);
        let token_hash = hex::encode(hasher.finalize());

        // Set expiry (1 hour normally, or in the past if expired)
        let expires_at = if expired {
            Utc::now() - Duration::hours(1)
        } else {
            Utc::now() + Duration::hours(1)
        };

        sqlx::query(
            r#"
            INSERT INTO password_reset_tokens (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.db)
        .await
        .expect("Failed to create password reset token");

        token
    }

    /// Clean up all created test data
    pub async fn cleanup(&self) {
        // Clean up books first (has FK to recipes)
        for book_id in &self.created_books {
            let _ = sqlx::query("DELETE FROM book_recipe_entries WHERE book_id = $1")
                .bind(book_id)
                .execute(&self.db)
                .await;
            let _ = sqlx::query("DELETE FROM recipe_books WHERE id = $1")
                .bind(book_id)
                .execute(&self.db)
                .await;
        }

        // Clean up recipes
        for recipe_id in &self.created_recipes {
            let _ = sqlx::query("DELETE FROM saved_recipes WHERE recipe_id = $1")
                .bind(recipe_id)
                .execute(&self.db)
                .await;
            let _ = sqlx::query("DELETE FROM book_recipe_entries WHERE recipe_id = $1")
                .bind(recipe_id)
                .execute(&self.db)
                .await;
            let _ = sqlx::query("DELETE FROM ingredients WHERE recipe_id = $1")
                .bind(recipe_id)
                .execute(&self.db)
                .await;
            let _ = sqlx::query("DELETE FROM instruction_steps WHERE recipe_id = $1")
                .bind(recipe_id)
                .execute(&self.db)
                .await;
            let _ = sqlx::query("DELETE FROM activities WHERE target_id = $1")
                .bind(recipe_id)
                .execute(&self.db)
                .await;
            let _ = sqlx::query("DELETE FROM recipes WHERE id = $1")
                .bind(recipe_id)
                .execute(&self.db)
                .await;
        }

        // Clean up users
        for user_id in &self.created_users {
            // Delete in order respecting foreign keys
            let tables = [
                "sessions",
                "email_confirmation_tokens",
                "password_reset_tokens",
                "recovery_codes",
                "security_events",
                "saved_recipes",
                "follows",
                "activities",
            ];

            for table in tables {
                let query = format!("DELETE FROM {} WHERE user_id = $1", table);
                let _ = sqlx::query(&query).bind(user_id).execute(&self.db).await;
            }

            // Delete recipes owned by user (in case not tracked)
            let _ = sqlx::query("DELETE FROM recipes WHERE author_id = $1")
                .bind(user_id)
                .execute(&self.db)
                .await;

            // Delete books owned by user (in case not tracked)
            let _ = sqlx::query("DELETE FROM recipe_books WHERE owner_id = $1")
                .bind(user_id)
                .execute(&self.db)
                .await;

            // Delete user
            let _ = sqlx::query("DELETE FROM users WHERE id = $1")
                .bind(user_id)
                .execute(&self.db)
                .await;
        }
    }

    // ==========================================================================
    // HTTP Request Helpers
    // ==========================================================================

    /// POST JSON to endpoint
    pub async fn post(&self, path: &str, body: Value) -> ApiResponse {
        let response = self.server.post(path).json(&body).await;

        let status = response.status_code().as_u16();
        let session_cookie = extract_session_cookie(&response);
        let text = response.text();
        let body = serde_json::from_str::<Value>(&text)
            .unwrap_or_else(|_| Value::Object(Default::default()));

        ApiResponse {
            status,
            body,
            session_cookie,
        }
    }

    /// GET endpoint
    pub async fn get(&self, path: &str) -> ApiResponse {
        let response = self.server.get(path).await;

        let status = response.status_code().as_u16();
        // Try to parse as JSON, fall back to empty object
        let text = response.text();
        let body = serde_json::from_str::<Value>(&text)
            .unwrap_or_else(|_| Value::Object(Default::default()));

        ApiResponse {
            status,
            body,
            session_cookie: None,
        }
    }

    /// GET with session cookie
    pub async fn get_with_session(&self, path: &str, session: &str) -> ApiResponse {
        let response = self
            .server
            .get(path)
            .add_cookie(cookie::Cookie::new(
                "oppskrift_session",
                session.to_string(),
            ))
            .await;

        let status = response.status_code().as_u16();
        let text = response.text();
        let body = serde_json::from_str::<Value>(&text)
            .unwrap_or_else(|_| Value::Object(Default::default()));

        ApiResponse {
            status,
            body,
            session_cookie: None,
        }
    }

    /// POST JSON with session cookie
    pub async fn post_with_session(&self, path: &str, body: Value, session: &str) -> ApiResponse {
        let response = self
            .server
            .post(path)
            .add_cookie(cookie::Cookie::new(
                "oppskrift_session",
                session.to_string(),
            ))
            .json(&body)
            .await;

        let status = response.status_code().as_u16();
        let session_cookie = extract_session_cookie(&response);
        let text = response.text();
        let body = serde_json::from_str::<Value>(&text)
            .unwrap_or_else(|_| Value::Object(Default::default()));

        ApiResponse {
            status,
            body,
            session_cookie,
        }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Note: async cleanup in Drop is tricky
        // Tests should call cleanup() explicitly
    }
}

/// Extract session cookie value from Set-Cookie header
fn extract_session_cookie(response: &axum_test::TestResponse) -> Option<String> {
    response
        .iter_headers_by_name("set-cookie")
        .find_map(|value| {
            let cookie_str = value.to_str().ok()?;
            if cookie_str.starts_with("oppskrift_session=") {
                // Extract the value between = and ;
                let start = "oppskrift_session=".len();
                let end = cookie_str.find(';').unwrap_or(cookie_str.len());
                Some(cookie_str[start..end].to_string())
            } else {
                None
            }
        })
}

/// API response wrapper
pub struct ApiResponse {
    pub status: u16,
    pub body: Value,
    pub session_cookie: Option<String>,
}

impl ApiResponse {
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.body.get(key)
    }

    pub fn error_message(&self) -> Option<&str> {
        self.body.get("message").and_then(|v| v.as_str())
    }
}

/// Generate a TOTP code from a base32 secret
pub fn generate_totp_code(secret_base32: &str) -> String {
    let secret = Secret::Encoded(secret_base32.to_string());
    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_bytes().unwrap(),
        None,
        "test@example.com".to_string(),
    )
    .expect("Failed to create TOTP");

    totp.generate_current()
        .expect("Failed to generate TOTP code")
}
