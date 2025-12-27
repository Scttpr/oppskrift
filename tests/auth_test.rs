//! Integration tests for authentication
//!
//! These tests require:
//! - A running PostgreSQL database (DATABASE_URL)
//! - The app server running (TEST_BASE_URL, defaults to localhost:3000)
//!
//! Run with: cargo test --test auth_test -- --test-threads=1

mod common;

use common::{ApiClient, TestContext};
use serde_json::json;

/// Test: Registration creates user and returns success
#[tokio::test]
async fn test_registration_success() {
    let mut ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();

    let response = client
        .post(
            "/api/v1/auth/register",
            json!({
                "email": email,
                "username": username,
                "password": "SecurePass123!",
                "display_name": "Test User"
            }),
        )
        .await;

    assert_eq!(
        response.status, 201,
        "Expected 201 Created: {:?}",
        response.body
    );
    assert!(
        response.get("user_id").is_some(),
        "Response should contain user_id"
    );
    assert!(
        response.get("message").is_some(),
        "Response should contain message"
    );

    // Track user for cleanup
    if let Some(user_id) = ctx.get_user_by_email(&email).await {
        ctx.track_user(user_id);
    }

    ctx.cleanup().await;
}

/// Test: Registration rejects duplicate email
#[tokio::test]
async fn test_registration_duplicate_email() {
    let mut ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let email = TestContext::unique_email();
    let username1 = TestContext::unique_username();
    let username2 = TestContext::unique_username();

    // First registration
    let response1 = client
        .post(
            "/api/v1/auth/register",
            json!({
                "email": email,
                "username": username1,
                "password": "SecurePass123!"
            }),
        )
        .await;
    assert!(response1.is_success(), "First registration should succeed");

    // Track user for cleanup
    if let Some(user_id) = ctx.get_user_by_email(&email).await {
        ctx.track_user(user_id);
    }

    // Second registration with same email
    let response2 = client
        .post(
            "/api/v1/auth/register",
            json!({
                "email": email,
                "username": username2,
                "password": "SecurePass123!"
            }),
        )
        .await;

    assert_eq!(
        response2.status, 409,
        "Expected 409 Conflict for duplicate email"
    );

    ctx.cleanup().await;
}

/// Test: Registration rejects invalid email format
#[tokio::test]
async fn test_registration_invalid_email() {
    let ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let response = client
        .post(
            "/api/v1/auth/register",
            json!({
                "email": "not-an-email",
                "username": TestContext::unique_username(),
                "password": "SecurePass123!"
            }),
        )
        .await;

    assert_eq!(response.status, 422, "Expected 422 for invalid email");
}

/// Test: Registration rejects short password
#[tokio::test]
async fn test_registration_short_password() {
    let ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let response = client
        .post(
            "/api/v1/auth/register",
            json!({
                "email": TestContext::unique_email(),
                "username": TestContext::unique_username(),
                "password": "short"
            }),
        )
        .await;

    assert_eq!(response.status, 422, "Expected 422 for short password");
}

/// Test: Registration rejects reserved username
#[tokio::test]
async fn test_registration_reserved_username() {
    let ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let response = client
        .post(
            "/api/v1/auth/register",
            json!({
                "email": TestContext::unique_email(),
                "username": "admin",
                "password": "SecurePass123!"
            }),
        )
        .await;

    assert_eq!(response.status, 409, "Expected 409 for reserved username");
}

/// Test: Login with valid credentials
#[tokio::test]
async fn test_login_success() {
    let mut ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let password = "SecurePass123!";

    // Create verified user directly in DB
    ctx.create_user(&email, &username, password, true).await;

    let response = client
        .post(
            "/api/v1/auth/login",
            json!({
                "email": email,
                "password": password
            }),
        )
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);
    assert!(
        response.get("user").is_some(),
        "Response should contain user"
    );
    assert!(
        response.get("expires_at").is_some(),
        "Response should contain expires_at"
    );

    ctx.cleanup().await;
}

/// Test: Login fails for unverified email
#[tokio::test]
async fn test_login_unverified_email() {
    let mut ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let password = "SecurePass123!";

    // Create unverified user
    ctx.create_user(&email, &username, password, false).await;

    let response = client
        .post(
            "/api/v1/auth/login",
            json!({
                "email": email,
                "password": password
            }),
        )
        .await;

    assert_eq!(
        response.status, 403,
        "Expected 403 for unverified email: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

/// Test: Login fails for wrong password
#[tokio::test]
async fn test_login_wrong_password() {
    let mut ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();

    // Create verified user
    ctx.create_user(&email, &username, "CorrectPass123!", true)
        .await;

    let response = client
        .post(
            "/api/v1/auth/login",
            json!({
                "email": email,
                "password": "WrongPass123!"
            }),
        )
        .await;

    assert_eq!(response.status, 401, "Expected 401 for wrong password");
    // Error message should be generic (no enumeration)
    assert_eq!(
        response.error_message(),
        Some("Invalid email or password"),
        "Error should be generic"
    );

    ctx.cleanup().await;
}

/// Test: Login fails for non-existent email (same error as wrong password)
#[tokio::test]
async fn test_login_nonexistent_email() {
    let ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let response = client
        .post(
            "/api/v1/auth/login",
            json!({
                "email": "nonexistent@example.com",
                "password": "SomePass123!"
            }),
        )
        .await;

    assert_eq!(response.status, 401, "Expected 401 for non-existent email");
    // Same error as wrong password (prevent enumeration)
    assert_eq!(
        response.error_message(),
        Some("Invalid email or password"),
        "Error should be generic (no enumeration)"
    );
}

/// Test: Health check endpoint
#[tokio::test]
async fn test_health_check() {
    let ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let response = client.get("/health").await;

    assert_eq!(response.status, 200, "Health check should return 200");
}

// =============================================================================
// Session Tests
// =============================================================================

/// Test: Login returns session cookie
#[tokio::test]
async fn test_login_returns_session_cookie() {
    let mut ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let password = "SecurePass123!";

    // Create verified user
    ctx.create_user(&email, &username, password, true).await;

    let response = client
        .post(
            "/api/v1/auth/login",
            json!({
                "email": email,
                "password": password
            }),
        )
        .await;

    assert_eq!(response.status, 200, "Login should succeed");
    assert!(
        response.session_cookie.is_some(),
        "Login should return session cookie"
    );

    ctx.cleanup().await;
}

/// Test: Access protected endpoint with valid session
#[tokio::test]
async fn test_access_protected_endpoint_with_session() {
    let mut ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let password = "SecurePass123!";

    // Create verified user and login
    ctx.create_user(&email, &username, password, true).await;

    let login_response = client
        .post(
            "/api/v1/auth/login",
            json!({
                "email": email,
                "password": password
            }),
        )
        .await;

    let session = login_response
        .session_cookie
        .expect("Login should return session");

    // Access /users/me with session
    let me_response = client.get_with_session("/api/v1/users/me", &session).await;

    assert_eq!(
        me_response.status, 200,
        "Should access protected endpoint: {:?}",
        me_response.body
    );
    assert_eq!(
        me_response.get("email").and_then(|v| v.as_str()),
        Some(email.as_str()),
        "Should return current user's email"
    );

    ctx.cleanup().await;
}

/// Test: Access protected endpoint without session returns 401
#[tokio::test]
async fn test_access_protected_endpoint_without_session() {
    let ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let response = client.get("/api/v1/users/me").await;

    assert_eq!(response.status, 401, "Should return 401 without session");
}

/// Test: Logout invalidates session
#[tokio::test]
async fn test_logout_invalidates_session() {
    let mut ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let password = "SecurePass123!";

    // Create verified user and login
    ctx.create_user(&email, &username, password, true).await;

    let login_response = client
        .post(
            "/api/v1/auth/login",
            json!({
                "email": email,
                "password": password
            }),
        )
        .await;

    let session = login_response
        .session_cookie
        .expect("Login should return session");

    // Logout
    let logout_response = client
        .post_with_session("/api/v1/auth/logout", json!({}), &session)
        .await;

    assert_eq!(logout_response.status, 200, "Logout should succeed");

    // Try to access protected endpoint with old session
    let me_response = client.get_with_session("/api/v1/users/me", &session).await;

    assert_eq!(
        me_response.status, 401,
        "Session should be invalid after logout"
    );

    ctx.cleanup().await;
}

/// Test: Logout without session returns 401
#[tokio::test]
async fn test_logout_without_session() {
    let ctx = TestContext::new().await;
    let client = ApiClient::new(&ctx.base_url);

    let response = client.post("/api/v1/auth/logout", json!({})).await;

    assert_eq!(
        response.status, 401,
        "Logout without session should return 401"
    );
}
