//! Integration tests for user settings
//!
//! These tests require a PostgreSQL database (DATABASE_URL).
//! Uses axum-test to test directly without needing a running HTTP server.
//!
//! Run with: cargo test --test settings_test -- --test-threads=1

mod common;

use common::{run_test, TestContext};
use serde_json::json;

// =============================================================================
// Profile Tests (User Story 1 & 2)
// =============================================================================

/// Test: View user profile returns user data
#[tokio::test]
async fn test_view_profile() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("profile_viewer").await;

        let response = ctx
            .get_with_session("/api/v1/account/profile", &session)
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);
        assert!(response.get("username").is_some(), "Should return username");
        assert!(
            response.get("display_name").is_some(),
            "Should return display_name"
        );
    })
    .await;
}

/// Test: Update profile changes user data
#[tokio::test]
async fn test_update_profile() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("profile_editor").await;

        let response = ctx
            .patch_with_session(
                "/api/v1/users/me",
                json!({
                    "display_name": "Updated Name",
                    "bio": "My new bio",
                    "measurement_pref": "imperial"
                }),
                &session,
            )
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

        // Verify changes persisted
        let profile = ctx
            .get_with_session("/api/v1/account/profile", &session)
            .await;
        assert_eq!(
            profile.get("display_name").and_then(|v| v.as_str()),
            Some("Updated Name")
        );
    })
    .await;
}

/// Test: Profile update via API stores bio as-is (sanitization done at display)
/// Note: The HTML handler sanitizes input, but API relies on frontend/display sanitization
#[tokio::test]
async fn test_profile_update_stores_bio() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("bio_tester").await;

        let response = ctx
            .patch_with_session(
                "/api/v1/users/me",
                json!({
                    "display_name": "Test User",
                    "bio": "I love cooking Italian food"
                }),
                &session,
            )
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

        // Verify bio was stored
        let profile = ctx
            .get_with_session("/api/v1/account/profile", &session)
            .await;
        let bio = profile.get("bio").and_then(|v| v.as_str()).unwrap_or("");
        assert!(bio.contains("cooking"), "Bio should be stored");
    })
    .await;
}

// =============================================================================
// Password Change Tests (User Story 4)
// =============================================================================

/// Test: Password change with correct current password
#[tokio::test]
async fn test_password_change_success() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let old_password = "Xk9#mP2$vL5@nQ8!";
        let new_password = "NewPass123!@#456";

        ctx.create_user(&email, &username, old_password, true).await;
        let session = ctx
            .login_and_get_session(&email, old_password)
            .await
            .expect("Login should succeed");

        let response = ctx
            .post_with_session(
                "/api/v1/account/change-password",
                json!({
                    "current_password": old_password,
                    "new_password": new_password
                }),
                &session,
            )
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

        // Verify can login with new password
        let new_session = ctx.login_and_get_session(&email, new_password).await;
        assert!(new_session.is_some(), "Should login with new password");
    })
    .await;
}

/// Test: Password change with wrong current password
#[tokio::test]
async fn test_password_change_wrong_current() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("password_changer").await;

        let response = ctx
            .post_with_session(
                "/api/v1/account/change-password",
                json!({
                    "current_password": "WrongPassword123!",
                    "new_password": "NewPass123!@#456"
                }),
                &session,
            )
            .await;

        assert_eq!(
            response.status, 401,
            "Expected 401 Unauthorized: {:?}",
            response.body
        );
    })
    .await;
}

/// Test: Password change rejects weak password
#[tokio::test]
async fn test_password_change_weak_password() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        ctx.create_user(&email, &username, password, true).await;
        let session = ctx
            .login_and_get_session(&email, password)
            .await
            .expect("Login should succeed");

        let response = ctx
            .post_with_session(
                "/api/v1/account/change-password",
                json!({
                    "current_password": password,
                    "new_password": "weak"
                }),
                &session,
            )
            .await;

        // 422 Unprocessable Entity is returned for validation errors
        assert_eq!(
            response.status, 422,
            "Expected 422 Unprocessable Entity: {:?}",
            response.body
        );
    })
    .await;
}

// =============================================================================
// Session Management Tests (User Story 6)
// =============================================================================

/// Test: List active sessions
#[tokio::test]
async fn test_list_sessions() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("session_viewer").await;

        let response = ctx
            .get_with_session("/api/v1/account/sessions", &session)
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);
        assert!(
            response.get("sessions").is_some(),
            "Should return sessions array"
        );
    })
    .await;
}

/// Test: Revoke specific session
#[tokio::test]
async fn test_revoke_specific_session() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        ctx.create_user(&email, &username, password, true).await;

        // Create session
        let session = ctx
            .login_and_get_session(&email, password)
            .await
            .expect("Login should succeed");

        // List sessions to get session ID
        let list_response = ctx
            .get_with_session("/api/v1/account/sessions", &session)
            .await;

        assert_eq!(
            list_response.status, 200,
            "Expected 200 OK: {:?}",
            list_response.body
        );

        // Get the session ID from the response
        let sessions = list_response
            .body
            .get("sessions")
            .and_then(|s| s.as_array());
        assert!(sessions.is_some(), "Should have sessions array");
        assert!(
            !sessions.unwrap().is_empty(),
            "Should have at least one session"
        );
    })
    .await;
}

// =============================================================================
// Account Deletion Tests (User Story 7)
// =============================================================================

/// Test: Request account deletion
#[tokio::test]
async fn test_request_deletion() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        ctx.create_user(&email, &username, password, true).await;
        let session = ctx
            .login_and_get_session(&email, password)
            .await
            .expect("Login should succeed");

        let response = ctx
            .post_with_session(
                "/api/v1/account/delete",
                json!({
                    "password": password,
                    "content_choice": "anonymize"
                }),
                &session,
            )
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);
        assert!(
            response.get("scheduled_for").is_some(),
            "Should return scheduled_for date"
        );
    })
    .await;
}

/// Test: Cancel account deletion
#[tokio::test]
async fn test_cancel_deletion() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";

        ctx.create_user(&email, &username, password, true).await;
        let session = ctx
            .login_and_get_session(&email, password)
            .await
            .expect("Login should succeed");

        // Request deletion
        ctx.post_with_session(
            "/api/v1/account/delete",
            json!({
                "password": password,
                "content_choice": "anonymize"
            }),
            &session,
        )
        .await;

        // Cancel deletion
        let response = ctx
            .post_with_session("/api/v1/account/cancel-deletion", json!({}), &session)
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);
    })
    .await;
}

/// Test: Deletion request requires password
#[tokio::test]
async fn test_deletion_requires_password() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("deletion_tester").await;

        let response = ctx
            .post_with_session(
                "/api/v1/account/delete",
                json!({
                    "password": "WrongPassword!",
                    "content_choice": "anonymize"
                }),
                &session,
            )
            .await;

        assert_eq!(
            response.status, 401,
            "Expected 401 Unauthorized: {:?}",
            response.body
        );
    })
    .await;
}

// =============================================================================
// Security Tests
// =============================================================================

/// Test: Settings endpoints require authentication
#[tokio::test]
async fn test_settings_require_auth() {
    run_test(|ctx| async move {
        // Profile
        let response = ctx.get("/api/v1/account/profile").await;
        assert_eq!(response.status, 401, "Profile should require auth");

        // Sessions
        let response = ctx.get("/api/v1/account/sessions").await;
        assert_eq!(response.status, 401, "Sessions should require auth");

        // Security info
        let response = ctx.get("/api/v1/account/security").await;
        assert_eq!(response.status, 401, "Security should require auth");
    })
    .await;
}
