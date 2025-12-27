//! Shared test utilities for integration tests
//!
//! These tests run against a real database. Ensure DATABASE_URL is set.

use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

/// Test context with database connection
pub struct TestContext {
    pub db: PgPool,
    pub base_url: String,
    created_users: Vec<Uuid>,
}

impl TestContext {
    /// Create a new test context
    pub async fn new() -> Self {
        dotenvy::dotenv().ok();

        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");

        let db = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database");

        let base_url =
            std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        Self {
            db,
            base_url,
            created_users: Vec::new(),
        }
    }

    /// Generate a unique test email
    pub fn unique_email() -> String {
        format!("test_{}@example.com", Uuid::new_v4())
    }

    /// Generate a unique test username
    pub fn unique_username() -> String {
        let id = Uuid::new_v4().to_string().replace("-", "");
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

        let ap_id = format!("{}/users/{}", self.base_url, username);

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
        for user_id in &self.created_users {
            // Delete in order respecting foreign keys
            let tables = [
                "sessions",
                "email_confirmation_tokens",
                "password_reset_tokens",
                "recovery_codes",
                "security_logs",
                "saved_recipes",
                "follows",
            ];

            for table in tables {
                let query = format!("DELETE FROM {} WHERE user_id = $1", table);
                let _ = sqlx::query(&query).bind(user_id).execute(&self.db).await;
            }

            // Delete user
            let _ = sqlx::query("DELETE FROM users WHERE id = $1")
                .bind(user_id)
                .execute(&self.db)
                .await;
        }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        // Note: async cleanup in Drop is tricky
        // Tests should call cleanup() explicitly
    }
}

/// HTTP client for API testing
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .cookie_store(true)
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.to_string(),
        }
    }

    /// POST JSON to endpoint
    pub async fn post(&self, path: &str, body: Value) -> ApiResponse {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .expect("Request failed");

        let status = response.status().as_u16();
        let session_cookie = extract_session_cookie(&response);
        let body = response.json().await.unwrap_or(json!({}));

        ApiResponse {
            status,
            body,
            session_cookie,
        }
    }

    /// GET endpoint
    pub async fn get(&self, path: &str) -> ApiResponse {
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.get(&url).send().await.expect("Request failed");

        ApiResponse {
            status: response.status().as_u16(),
            body: response.json().await.unwrap_or(json!({})),
            session_cookie: None,
        }
    }

    /// GET with session cookie
    pub async fn get_with_session(&self, path: &str, session: &str) -> ApiResponse {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .header("Cookie", format!("oppskrift_session={}", session))
            .send()
            .await
            .expect("Request failed");

        ApiResponse {
            status: response.status().as_u16(),
            body: response.json().await.unwrap_or(json!({})),
            session_cookie: None,
        }
    }

    /// POST JSON with session cookie
    pub async fn post_with_session(&self, path: &str, body: Value, session: &str) -> ApiResponse {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .header("Cookie", format!("oppskrift_session={}", session))
            .json(&body)
            .send()
            .await
            .expect("Request failed");

        let status = response.status().as_u16();
        let session_cookie = extract_session_cookie(&response);
        let body = response.json().await.unwrap_or(json!({}));

        ApiResponse {
            status,
            body,
            session_cookie,
        }
    }
}

/// Extract session cookie value from Set-Cookie header
fn extract_session_cookie(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get_all("set-cookie")
        .iter()
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
