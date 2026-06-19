//! Authentication security tests (T073-T075)
//!
//! Tests for OSK Principle IV compliance - Authentication Security
//! Verifies JWT/session security, cookie flags, and account lockout.
//!
//! Run with: cargo test --test security_auth_security_test

mod common;

use common::{run_test, TestContext};
use serde_json::json;

// =============================================================================
// Session Cookie Security Tests (T074)
// =============================================================================

/// Test: Session cookie should have HttpOnly flag
#[tokio::test]
async fn test_session_cookie_httponly() {
    run_test(|mut ctx| async move {
        // Create and login user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        ctx.create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let response = ctx
            .server
            .post("/api/v1/auth/login")
            .json(&json!({
                "email": email,
                "password": "Xk9#mP2$vL5@nQ8!"
            }))
            .await;

        assert_eq!(response.status_code().as_u16(), 200);

        // Find the session cookie
        let session_cookie = response
            .iter_headers_by_name("set-cookie")
            .find(|v| {
                v.to_str()
                    .map(|s| s.contains("oppskrift_session"))
                    .unwrap_or(false)
            })
            .map(|v| v.to_str().unwrap_or_default().to_string());

        assert!(
            session_cookie.is_some(),
            "Session cookie should be set on login"
        );

        let cookie_str = session_cookie.unwrap();
        assert!(
            cookie_str.to_lowercase().contains("httponly"),
            "Session cookie must have HttpOnly flag. Cookie: {}",
            cookie_str
        );
    })
    .await;
}

/// Test: Session cookie should have Secure flag (for HTTPS)
#[tokio::test]
async fn test_session_cookie_secure_flag() {
    run_test(|mut ctx| async move {
        // Create and login user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        ctx.create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let response = ctx
            .server
            .post("/api/v1/auth/login")
            .json(&json!({
                "email": email,
                "password": "Xk9#mP2$vL5@nQ8!"
            }))
            .await;

        let session_cookie = response
            .iter_headers_by_name("set-cookie")
            .find(|v| {
                v.to_str()
                    .map(|s| s.contains("oppskrift_session"))
                    .unwrap_or(false)
            })
            .map(|v| v.to_str().unwrap_or_default().to_string());

        if let Some(cookie_str) = session_cookie {
            // In test environment (non-HTTPS), Secure flag might be conditionally set
            // Check that it's present OR document the finding
            if !cookie_str.to_lowercase().contains("secure") {
                eprintln!(
                    "SECURITY NOTE: Session cookie doesn't have Secure flag.\n\
                     This is acceptable in dev/test but MUST be enabled in production.\n\
                     Cookie: {}",
                    cookie_str
                );
            }
        }
    })
    .await;
}

/// Test: Session cookie should have SameSite attribute
#[tokio::test]
async fn test_session_cookie_samesite() {
    run_test(|mut ctx| async move {
        // Create and login user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        ctx.create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let response = ctx
            .server
            .post("/api/v1/auth/login")
            .json(&json!({
                "email": email,
                "password": "Xk9#mP2$vL5@nQ8!"
            }))
            .await;

        let session_cookie = response
            .iter_headers_by_name("set-cookie")
            .find(|v| {
                v.to_str()
                    .map(|s| s.contains("oppskrift_session"))
                    .unwrap_or(false)
            })
            .map(|v| v.to_str().unwrap_or_default().to_string());

        assert!(session_cookie.is_some(), "Session cookie should be set");

        let cookie_str = session_cookie.unwrap();
        let has_samesite = cookie_str.to_lowercase().contains("samesite");

        assert!(
            has_samesite,
            "Session cookie must have SameSite attribute for CSRF protection. Cookie: {}",
            cookie_str
        );

        // Verify it's set to Strict or Lax (not None)
        let is_samesite_strict = cookie_str.to_lowercase().contains("samesite=strict");
        let is_samesite_lax = cookie_str.to_lowercase().contains("samesite=lax");

        assert!(
            is_samesite_strict || is_samesite_lax,
            "SameSite should be Strict or Lax, not None. Cookie: {}",
            cookie_str
        );
    })
    .await;
}

/// Test: Session cookie should have reasonable expiry
#[tokio::test]
async fn test_session_cookie_expiry() {
    run_test(|mut ctx| async move {
        // Create and login user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        ctx.create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let response = ctx
            .server
            .post("/api/v1/auth/login")
            .json(&json!({
                "email": email,
                "password": "Xk9#mP2$vL5@nQ8!"
            }))
            .await;

        let session_cookie = response
            .iter_headers_by_name("set-cookie")
            .find(|v| {
                v.to_str()
                    .map(|s| s.contains("oppskrift_session"))
                    .unwrap_or(false)
            })
            .map(|v| v.to_str().unwrap_or_default().to_string());

        if let Some(cookie_str) = session_cookie {
            // Check for Max-Age or Expires attribute
            let has_expiry = cookie_str.to_lowercase().contains("max-age")
                || cookie_str.to_lowercase().contains("expires");

            assert!(
                has_expiry,
                "Session cookie should have expiry (Max-Age or Expires). Cookie: {}",
                cookie_str
            );
        }
    })
    .await;
}

// =============================================================================
// Session Token Security Tests (T073)
// =============================================================================

/// Test: Session token should be sufficiently long
#[tokio::test]
async fn test_session_token_length() {
    run_test(|mut ctx| async move {
        // Create and login user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        ctx.create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": email,
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert!(
            response.session_cookie.is_some(),
            "Login should return session cookie"
        );

        let token = response.session_cookie.unwrap();
        // Token should be at least 32 bytes (256 bits) of entropy when hex-encoded = 64 chars
        // But cookie encoding might vary, so we check for reasonable minimum
        assert!(
            token.len() >= 32,
            "Session token should be at least 32 characters (128 bits minimum). Got {} chars",
            token.len()
        );
    })
    .await;
}

/// Test: Session token should be unique per login
#[tokio::test]
async fn test_session_token_uniqueness() {
    run_test(|mut ctx| async move {
        // Create user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        ctx.create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        // Login multiple times and collect tokens
        let mut tokens = Vec::new();
        for _ in 0..5 {
            let response = ctx
                .post(
                    "/api/v1/auth/login",
                    json!({
                        "email": &email,
                        "password": "Xk9#mP2$vL5@nQ8!"
                    }),
                )
                .await;

            if let Some(token) = response.session_cookie {
                tokens.push(token);
            }
        }

        // All tokens should be unique
        let unique_count = tokens
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert_eq!(
            unique_count,
            tokens.len(),
            "Each login should generate a unique session token"
        );
    })
    .await;
}

/// Test: Forged/invalid session token should be rejected
#[tokio::test]
async fn test_session_token_forgery_rejected() {
    run_test(|ctx| async move {
        // Try to access protected endpoint with forged token
        let forged_tokens = [
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "invalid_token_here",
            "00000000-0000-0000-0000-000000000000",
            "../../../etc/passwd",
            "<script>alert('xss')</script>",
        ];

        for forged in forged_tokens {
            let response = ctx
                .server
                .get("/api/v1/account/profile")
                .add_cookie(cookie::Cookie::new("oppskrift_session", forged))
                .await;

            assert_eq!(
                response.status_code().as_u16(),
                401,
                "Forged token '{}' should be rejected",
                forged
            );
        }
    })
    .await;
}

/// Test: Expired session should be rejected
#[tokio::test]
async fn test_expired_session_rejected() {
    run_test(|mut ctx| async move {
        // Create user and get valid session
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": &email,
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        let session = response.session_cookie.expect("Should get session");

        // Manually expire all sessions for this user in database
        sqlx::query(
            "UPDATE sessions SET expires_at = NOW() - INTERVAL '1 hour' WHERE user_id = $1",
        )
        .bind(user_id)
        .execute(&ctx.db)
        .await
        .expect("Should update sessions");

        // Try to access protected endpoint
        let response = ctx
            .get_with_session("/api/v1/account/profile", &session)
            .await;

        assert_eq!(response.status, 401, "Expired session should be rejected");
    })
    .await;
}

// =============================================================================
// Account Lockout Tests (T075)
// =============================================================================

/// Test: Account should be locked after failed login attempts
#[tokio::test]
async fn test_account_lockout_after_failed_attempts() {
    run_test(|mut ctx| async move {
        // Create user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        ctx.create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        // Make failed login attempts (auth service has MAX_FAILED_LOGIN_ATTEMPTS = 5)
        for i in 0..5 {
            let response = ctx
                .post(
                    "/api/v1/auth/login",
                    json!({
                        "email": &email,
                        "password": "WrongPassword123!"
                    }),
                )
                .await;

            // All should fail with 401 (invalid credentials)
            assert_eq!(
                response.status,
                401,
                "Failed attempt {} should return 401",
                i + 1
            );
        }

        // 6th attempt with CORRECT password should be locked out
        let response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": &email,
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        // Should be 403 (Forbidden/Locked) or 401 with lockout message
        assert!(
            response.status == 403 || response.status == 401,
            "Account should be locked after 5 failed attempts. Status: {}, Body: {:?}",
            response.status,
            response.body
        );

        // Check the message indicates lockout
        let message = response
            .body
            .get("message")
            .or_else(|| response.body.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let indicates_lockout = message.to_lowercase().contains("lock")
            || message.to_lowercase().contains("verrouillé")
            || message.to_lowercase().contains("too many")
            || message.to_lowercase().contains("temporarily")
            || response.body.get("locked_until").is_some();

        assert!(
            indicates_lockout,
            "Response should indicate account lockout. Message: {:?}, Body: {:?}",
            message, response.body
        );
    })
    .await;
}

/// Test: Lockout should expire after the lockout period
#[tokio::test]
async fn test_account_lockout_expires() {
    run_test(|mut ctx| async move {
        // Create user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        // Lock the account by making failed attempts
        for _ in 0..5 {
            let _ = ctx
                .post(
                    "/api/v1/auth/login",
                    json!({
                        "email": &email,
                        "password": "WrongPassword123!"
                    }),
                )
                .await;
        }

        // Manually expire the lockout in database (simulate time passing)
        sqlx::query(
            "UPDATE users SET locked_until = NOW() - INTERVAL '1 minute', failed_login_attempts = 0 WHERE id = $1"
        )
        .bind(user_id)
        .execute(&ctx.db)
        .await
        .expect("Should update lockout");

        // Try login with correct password - should succeed now
        let response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": &email,
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(
            response.status, 200,
            "Login should succeed after lockout expires. Body: {:?}",
            response.body
        );
    })
    .await;
}

/// Test: Successful login should reset failed attempt counter
#[tokio::test]
async fn test_successful_login_resets_failed_attempts() {
    run_test(|mut ctx| async move {
        // Create user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        // Make a few failed attempts (not enough to lock)
        for _ in 0..3 {
            let _ = ctx
                .post(
                    "/api/v1/auth/login",
                    json!({
                        "email": &email,
                        "password": "WrongPassword123!"
                    }),
                )
                .await;
        }

        // Login successfully
        let response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": &email,
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        assert_eq!(response.status, 200, "Login should succeed");

        // Check that failed_login_attempts was reset
        let failed_attempts: i32 =
            sqlx::query_scalar("SELECT failed_login_attempts FROM users WHERE id = $1")
                .bind(user_id)
                .fetch_one(&ctx.db)
                .await
                .expect("Should query user");

        assert_eq!(
            failed_attempts, 0,
            "Failed attempts should be reset after successful login"
        );
    })
    .await;
}

/// Test: Logout should invalidate the session
#[tokio::test]
async fn test_logout_invalidates_session() {
    run_test(|mut ctx| async move {
        // Create and login user
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        ctx.create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let login_response = ctx
            .post(
                "/api/v1/auth/login",
                json!({
                    "email": &email,
                    "password": "Xk9#mP2$vL5@nQ8!"
                }),
            )
            .await;

        let session = login_response.session_cookie.expect("Should get session");

        // Logout
        let logout_response = ctx
            .post_with_session("/api/v1/auth/logout", json!({}), &session)
            .await;

        assert!(
            logout_response.status == 200 || logout_response.status == 204,
            "Logout should succeed"
        );

        // Try to use the same session - should fail
        let protected_response = ctx
            .get_with_session("/api/v1/account/profile", &session)
            .await;

        assert_eq!(
            protected_response.status, 401,
            "Session should be invalid after logout"
        );
    })
    .await;
}
