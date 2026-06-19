//! Integration tests for authentication
//!
//! These tests require a PostgreSQL database (DATABASE_URL).
//! Uses axum-test to test directly without needing a running HTTP server.
//!
//! Run with: cargo test --test auth_test -- --test-threads=1

mod common;

use common::{generate_totp_code, run_test, TestContext};
use serde_json::json;

/// Test: Registration creates user and returns success
#[tokio::test]
async fn test_registration_success() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();

        let response = ctx
            .post(
                "/api/v1/auth/register",
                json!({
                    "email": email,
                    "username": username,
                    "password": "Xk9#mP2$vL5@nQ8!",
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
    })
    .await;
}

/// Test: Registration rejects duplicate email
#[tokio::test]
async fn test_registration_duplicate_email() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username1 = TestContext::unique_username();
        let username2 = TestContext::unique_username();

        // First registration
        let response1 = ctx
            .post(
                "/api/v1/auth/register",
                json!({
                    "email": email,
                    "username": username1,
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;
        assert!(response1.is_success(), "First registration should succeed");

        // Track user for cleanup
        if let Some(user_id) = ctx.get_user_by_email(&email).await {
            ctx.track_user(user_id);
        }

        // Second registration with same email
        let response2 = ctx
            .post(
                "/api/v1/auth/register",
                json!({
                    "email": email,
                    "username": username2,
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(
            response2.status, 409,
            "Expected 409 Conflict for duplicate email"
        );
    })
    .await;
}

/// Test: Registration rejects invalid email format
#[tokio::test]
async fn test_registration_invalid_email() {
    run_test(|ctx| async move {
        let response = ctx
            .post(
                "/api/v1/auth/register",
                json!({
                    "email": "not-an-email",
                    "username": TestContext::unique_username(),
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(response.status, 422, "Expected 422 for invalid email");
    })
    .await;
}

/// Test: Registration rejects short password
#[tokio::test]
async fn test_registration_short_password() {
    run_test(|ctx| async move {
        let response = ctx
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
    })
    .await;
}

/// Test: Registration rejects reserved username
#[tokio::test]
async fn test_registration_reserved_username() {
    run_test(|ctx| async move {
        let response = ctx
            .post(
                "/api/v1/auth/register",
                json!({
                    "email": TestContext::unique_email(),
                    "username": "admin",
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(response.status, 400, "Expected 400 for reserved username");
    })
    .await;
}

/// Test: Login with valid credentials
#[tokio::test]
async fn test_login_success() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create verified user directly in DB
        ctx.create_user(&email, &username, password, true).await;

        let response = ctx
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
    })
    .await;
}

/// Test: Login fails for unverified email
#[tokio::test]
async fn test_login_unverified_email() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create unverified user
        ctx.create_user(&email, &username, password, false).await;

        let response = ctx
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
    })
    .await;
}

/// Test: Login fails for wrong password
#[tokio::test]
async fn test_login_wrong_password() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();

        // Create verified user
        ctx.create_user(&email, &username, "CorrectPass123!", true)
            .await;

        let response = ctx
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
            Some("E-mail ou mot de passe incorrect"),
            "Error should be generic"
        );
    })
    .await;
}

/// Test: Login fails for non-existent email (same error as wrong password)
#[tokio::test]
async fn test_login_nonexistent_email() {
    run_test(|ctx| async move {
        let response = ctx
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
            Some("E-mail ou mot de passe incorrect"),
            "Error should be generic (no enumeration)"
        );
    })
    .await;
}

/// Test: Health check endpoint
#[tokio::test]
async fn test_health_check() {
    run_test(|ctx| async move {
        let response = ctx.get("/health").await;

        assert_eq!(response.status, 200, "Health check should return 200");
    })
    .await;
}

// =============================================================================
// Session Tests
// =============================================================================

/// Test: Login returns session cookie
#[tokio::test]
async fn test_login_returns_session_cookie() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create verified user
        ctx.create_user(&email, &username, password, true).await;

        let response = ctx
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
    })
    .await;
}

/// Test: Access protected endpoint with valid session
#[tokio::test]
async fn test_access_protected_endpoint_with_session() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create verified user and login
        ctx.create_user(&email, &username, password, true).await;

        let login_response = ctx
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
        let me_response = ctx.get_with_session("/api/v1/users/me", &session).await;

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
    })
    .await;
}

/// Test: Access protected endpoint without session returns 401
#[tokio::test]
async fn test_access_protected_endpoint_without_session() {
    run_test(|ctx| async move {
        let response = ctx.get("/api/v1/users/me").await;

        assert_eq!(response.status, 401, "Should return 401 without session");
    })
    .await;
}

/// Test: Logout invalidates session
#[tokio::test]
async fn test_logout_invalidates_session() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create verified user and login
        ctx.create_user(&email, &username, password, true).await;

        let login_response = ctx
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
        let logout_response = ctx
            .post_with_session("/api/v1/auth/logout", json!({}), &session)
            .await;

        assert_eq!(logout_response.status, 200, "Logout should succeed");

        // Try to access protected endpoint with old session
        let me_response = ctx.get_with_session("/api/v1/users/me", &session).await;

        assert_eq!(
            me_response.status, 401,
            "Session should be invalid after logout"
        );
    })
    .await;
}

/// Test: Logout without session returns 401
#[tokio::test]
async fn test_logout_without_session() {
    run_test(|ctx| async move {
        let response = ctx.post("/api/v1/auth/logout", json!({})).await;

        assert_eq!(
            response.status, 401,
            "Logout without session should return 401"
        );
    })
    .await;
}

// =============================================================================
// Password Reset Tests
// =============================================================================

/// Test: Forgot password always returns success (prevents enumeration)
#[tokio::test]
async fn test_forgot_password_success() {
    run_test(|ctx| async move {
        let response = ctx
            .post(
                "/api/v1/auth/forgot-password",
                json!({
                    "email": "nonexistent@example.com"
                }),
            )
            .await;

        // Always returns 200 to prevent email enumeration
        assert_eq!(
            response.status, 200,
            "Forgot password should always return 200"
        );
    })
    .await;
}

/// Test: Reset password with valid token
#[tokio::test]
async fn test_reset_password_success() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();

        // Create user
        let user_id = ctx
            .create_user(&email, &username, "OldPassword123!", true)
            .await;

        // Create reset token
        let token = ctx.create_password_reset_token(user_id, false).await;

        // Reset password
        let response = ctx
            .post(
                "/api/v1/auth/reset-password",
                json!({
                    "token": token,
                    "new_password": "NewXk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(
            response.status, 200,
            "Reset password should succeed: {:?}",
            response.body
        );

        // Verify can login with new password
        let login_response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": email,
                    "password": "NewXk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(login_response.status, 200, "Should login with new password");
    })
    .await;
}

/// Test: Reset password with invalid token
#[tokio::test]
async fn test_reset_password_invalid_token() {
    run_test(|ctx| async move {
        let response = ctx
            .post(
                "/api/v1/auth/reset-password",
                json!({
                    "token": "0000000000000000000000000000000000000000000000000000000000000000",
                    "new_password": "NewXk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(response.status, 400, "Invalid token should return 400");
    })
    .await;
}

/// Test: Reset password with expired token
#[tokio::test]
async fn test_reset_password_expired_token() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();

        // Create user
        let user_id = ctx
            .create_user(&email, &username, "OldPassword123!", true)
            .await;

        // Create expired token
        let token = ctx.create_password_reset_token(user_id, true).await;

        let response = ctx
            .post(
                "/api/v1/auth/reset-password",
                json!({
                    "token": token,
                    "new_password": "NewXk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(response.status, 400, "Expired token should return 400");
    })
    .await;
}

/// Test: Reset password with weak password
#[tokio::test]
async fn test_reset_password_weak_password() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();

        // Create user
        let user_id = ctx
            .create_user(&email, &username, "OldPassword123!", true)
            .await;

        // Create reset token
        let token = ctx.create_password_reset_token(user_id, false).await;

        let response = ctx
            .post(
                "/api/v1/auth/reset-password",
                json!({
                    "token": token,
                    "new_password": "weak"
                }),
            )
            .await;

        assert_eq!(response.status, 422, "Weak password should return 422");
    })
    .await;
}

// =============================================================================
// Email Confirmation Tests
// =============================================================================

/// Test: Confirm email with valid token
#[tokio::test]
async fn test_confirm_email_success() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();

        // Create unverified user
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", false)
            .await;

        // Create confirmation token
        let token = ctx
            .create_email_confirmation_token(user_id, &email, false)
            .await;

        // Confirm email
        let response = ctx
            .get(&format!("/api/v1/auth/confirm-email/{}", token))
            .await;

        assert_eq!(
            response.status, 200,
            "Confirm email should succeed: {:?}",
            response.body
        );

        // Verify can now login
        let login_response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": email,
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(
            login_response.status, 200,
            "Should be able to login after email confirmation"
        );
    })
    .await;
}

/// Test: Confirm email with invalid token
#[tokio::test]
async fn test_confirm_email_invalid_token() {
    run_test(|ctx| async move {
        let response = ctx
            .get("/api/v1/auth/confirm-email/0000000000000000000000000000000000000000000000000000000000000000")
            .await;

        assert_eq!(response.status, 400, "Invalid token should return 400");
    })
    .await;
}

/// Test: Confirm email with expired token
#[tokio::test]
async fn test_confirm_email_expired_token() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();

        // Create unverified user
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", false)
            .await;

        // Create expired token
        let token = ctx
            .create_email_confirmation_token(user_id, &email, true)
            .await;

        let response = ctx
            .get(&format!("/api/v1/auth/confirm-email/{}", token))
            .await;

        assert_eq!(response.status, 400, "Expired token should return 400");
    })
    .await;
}

/// Test: Confirm email when already verified
#[tokio::test]
async fn test_confirm_email_already_verified() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();

        // Create already verified user
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        // Create confirmation token anyway
        let token = ctx
            .create_email_confirmation_token(user_id, &email, false)
            .await;

        let response = ctx
            .get(&format!("/api/v1/auth/confirm-email/{}", token))
            .await;

        assert_eq!(
            response.status, 409,
            "Already verified should return 409 Conflict"
        );
    })
    .await;
}

/// Test: Resend confirmation always returns success (prevents enumeration)
#[tokio::test]
async fn test_resend_confirmation_success() {
    run_test(|ctx| async move {
        let response = ctx
            .post(
                "/api/v1/auth/resend-confirmation",
                json!({
                    "email": "nonexistent@example.com"
                }),
            )
            .await;

        // Always returns 200 to prevent email enumeration
        assert_eq!(
            response.status, 200,
            "Resend confirmation should always return 200"
        );
    })
    .await;
}

// =============================================================================
// Two-Factor Authentication Tests
// =============================================================================

/// Test: 2FA setup requires authentication
#[tokio::test]
async fn test_2fa_setup_requires_auth() {
    run_test(|ctx| async move {
        let response = ctx.post("/api/v1/account/2fa/setup", json!({})).await;

        assert_eq!(
            response.status, 401,
            "2FA setup should require authentication"
        );
    })
    .await;
}

/// Test: 2FA setup returns secret and QR code
#[tokio::test]
async fn test_2fa_setup_success() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create verified user and login
        ctx.create_user(&email, &username, password, true).await;

        let login_response = ctx
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

        // Setup 2FA
        let response = ctx
            .post_with_session("/api/v1/account/2fa/setup", json!({}), &session)
            .await;

        assert_eq!(
            response.status, 200,
            "2FA setup should succeed: {:?}",
            response.body
        );
        assert!(response.get("secret").is_some(), "Should return secret");
        assert!(response.get("qr_code").is_some(), "Should return QR code");
    })
    .await;
}

/// Test: 2FA enable with valid code
#[tokio::test]
async fn test_2fa_enable_success() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create verified user and login
        ctx.create_user(&email, &username, password, true).await;

        let login_response = ctx
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

        // Setup 2FA
        let setup_response = ctx
            .post_with_session("/api/v1/account/2fa/setup", json!({}), &session)
            .await;

        let secret = setup_response
            .get("secret")
            .and_then(|v| v.as_str())
            .expect("Should have secret");

        // Generate valid TOTP code
        let totp_code = generate_totp_code(secret);

        // Enable 2FA
        let enable_response = ctx
            .post_with_session(
                "/api/v1/account/2fa/enable",
                json!({ "totp_code": totp_code }),
                &session,
            )
            .await;

        assert_eq!(
            enable_response.status, 200,
            "2FA enable should succeed: {:?}",
            enable_response.body
        );
        assert!(
            enable_response.get("recovery_codes").is_some(),
            "Should return recovery codes"
        );
    })
    .await;
}

/// Test: 2FA enable with invalid code fails
#[tokio::test]
async fn test_2fa_enable_invalid_code() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create verified user and login
        ctx.create_user(&email, &username, password, true).await;

        let login_response = ctx
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

        // Setup 2FA
        ctx.post_with_session("/api/v1/account/2fa/setup", json!({}), &session)
            .await;

        // Try to enable with invalid code
        let enable_response = ctx
            .post_with_session(
                "/api/v1/account/2fa/enable",
                json!({ "totp_code": "000000" }),
                &session,
            )
            .await;

        assert_eq!(
            enable_response.status, 400,
            "Invalid TOTP code should return 400"
        );
    })
    .await;
}

/// Test: Login with 2FA enabled returns partial token
#[tokio::test]
async fn test_login_with_2fa_returns_partial_token() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create verified user and login
        ctx.create_user(&email, &username, password, true).await;

        let login_response = ctx
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

        // Setup and enable 2FA
        let setup_response = ctx
            .post_with_session("/api/v1/account/2fa/setup", json!({}), &session)
            .await;

        let secret = setup_response
            .get("secret")
            .and_then(|v| v.as_str())
            .expect("Should have secret");

        let totp_code = generate_totp_code(secret);

        ctx.post_with_session(
            "/api/v1/account/2fa/enable",
            json!({ "totp_code": totp_code }),
            &session,
        )
        .await;

        // Now login again - should require 2FA
        let login_2fa_response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": email,
                    "password": password
                }),
            )
            .await;

        assert_eq!(login_2fa_response.status, 200, "Login should return 200");
        assert_eq!(
            login_2fa_response
                .get("requires_2fa")
                .and_then(|v| v.as_bool()),
            Some(true),
            "Should indicate 2FA required"
        );
        assert!(
            login_2fa_response.get("partial_token").is_some(),
            "Should return partial token"
        );
    })
    .await;
}

/// Test: 2FA verify completes login
#[tokio::test]
async fn test_2fa_verify_completes_login() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        // Create verified user and login
        ctx.create_user(&email, &username, password, true).await;

        let login_response = ctx
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

        // Setup and enable 2FA
        let setup_response = ctx
            .post_with_session("/api/v1/account/2fa/setup", json!({}), &session)
            .await;

        let secret = setup_response
            .get("secret")
            .and_then(|v| v.as_str())
            .expect("Should have secret")
            .to_string();

        let totp_code = generate_totp_code(&secret);

        ctx.post_with_session(
            "/api/v1/account/2fa/enable",
            json!({ "totp_code": totp_code }),
            &session,
        )
        .await;

        // Login again - get partial token
        let login_2fa_response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": email,
                    "password": password
                }),
            )
            .await;

        let partial_token = login_2fa_response
            .get("partial_token")
            .and_then(|v| v.as_str())
            .expect("Should have partial token");

        // Generate fresh TOTP code and verify
        let totp_code = generate_totp_code(&secret);

        let verify_response = ctx
            .post(
                "/api/v1/auth/2fa/verify",
                json!({
                    "partial_token": partial_token,
                    "totp_code": totp_code
                }),
            )
            .await;

        assert_eq!(
            verify_response.status, 200,
            "2FA verify should succeed: {:?}",
            verify_response.body
        );
        assert!(
            verify_response.session_cookie.is_some(),
            "Should return session cookie"
        );
    })
    .await;
}
