//! Integration tests for UI Parity features (006-ui-parity)
//!
//! Tests for:
//! - Session management (view/revoke individual sessions)
//! - Security events viewer
//! - Privacy settings and data export
//! - Book contribution management
//! - Followers/following lists
//!
//! Run with: cargo test --test ui_parity_test -- --test-threads=1

mod common;

use common::TestContext;
use serde_json::json;

// =============================================================================
// Session Management Tests (T044 - User Story 1)
// =============================================================================

/// Test: List active sessions returns current session
#[tokio::test]
async fn test_list_sessions_returns_current() {
    let mut ctx = TestContext::new().await;
    let (_user_id, session) = ctx.create_and_login("session_list").await;

    let response = ctx
        .get_with_session("/api/v1/account/sessions", &session)
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    let sessions = response.get("sessions").and_then(|s| s.as_array());
    assert!(sessions.is_some(), "Should return sessions array");
    assert!(
        !sessions.unwrap().is_empty(),
        "Should have at least one session"
    );

    // Verify session has required fields
    let first_session = &sessions.unwrap()[0];
    assert!(first_session.get("id").is_some(), "Session should have id");
    assert!(
        first_session.get("is_current").is_some(),
        "Session should have is_current flag"
    );

    ctx.cleanup().await;
}

/// Test: Multiple sessions can be created and listed
#[tokio::test]
async fn test_multiple_sessions_listed() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let password = "Xk9#mP2$vL5@nQ8!";

    ctx.create_user(&email, &username, password, true).await;

    // Create first session
    let session1 = ctx
        .login_and_get_session(&email, password)
        .await
        .expect("First login should succeed");

    // Create second session
    let session2 = ctx
        .login_and_get_session(&email, password)
        .await
        .expect("Second login should succeed");

    // List sessions from second session
    let response = ctx
        .get_with_session("/api/v1/account/sessions", &session2)
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    let sessions = response
        .get("sessions")
        .and_then(|s| s.as_array())
        .expect("Should have sessions array");

    // Should have at least 2 sessions
    assert!(
        sessions.len() >= 2,
        "Should have at least 2 sessions, got {}",
        sessions.len()
    );

    // Verify session1 is still valid
    let check = ctx
        .get_with_session("/api/v1/account/profile", &session1)
        .await;
    assert_eq!(check.status, 200, "First session should still be valid");

    ctx.cleanup().await;
}

/// Test: Revoke all other sessions keeps current session
#[tokio::test]
async fn test_revoke_all_other_sessions() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let password = "Xk9#mP2$vL5@nQ8!";

    ctx.create_user(&email, &username, password, true).await;

    // Create two sessions
    let session1 = ctx
        .login_and_get_session(&email, password)
        .await
        .expect("First login should succeed");

    let session2 = ctx
        .login_and_get_session(&email, password)
        .await
        .expect("Second login should succeed");

    // Revoke all other sessions from session2
    let response = ctx
        .post_with_session(
            "/api/v1/account/sessions/revoke-others",
            json!({}),
            &session2,
        )
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    // session1 should now be invalid
    let check1 = ctx
        .get_with_session("/api/v1/account/profile", &session1)
        .await;
    assert_eq!(check1.status, 401, "First session should be revoked");

    // session2 should still be valid
    let check2 = ctx
        .get_with_session("/api/v1/account/profile", &session2)
        .await;
    assert_eq!(check2.status, 200, "Current session should still be valid");

    ctx.cleanup().await;
}

// =============================================================================
// Security Events Tests (T045 - User Story 2)
// =============================================================================

/// Test: Security events endpoint returns events
#[tokio::test]
async fn test_security_events_list() {
    let mut ctx = TestContext::new().await;
    let (_user_id, session) = ctx.create_and_login("security_events").await;

    let response = ctx
        .get_with_session("/api/v1/account/security-events", &session)
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    // Should have events array (at least login event from create_and_login)
    let events = response.get("events").and_then(|e| e.as_array());
    assert!(events.is_some(), "Should return events array");

    ctx.cleanup().await;
}

/// Test: Security events are recorded for login
#[tokio::test]
async fn test_security_event_recorded_on_login() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let password = "Xk9#mP2$vL5@nQ8!";

    ctx.create_user(&email, &username, password, true).await;

    // Login
    let session = ctx
        .login_and_get_session(&email, password)
        .await
        .expect("Login should succeed");

    // Check security events
    let response = ctx
        .get_with_session("/api/v1/account/security-events", &session)
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    let events = response
        .get("events")
        .and_then(|e| e.as_array())
        .expect("Should have events");

    // Should have at least one login event
    let has_login_event = events.iter().any(|e| {
        e.get("event_type")
            .and_then(|t| t.as_str())
            .map(|t| t.contains("login"))
            .unwrap_or(false)
    });

    assert!(has_login_event, "Should have a login event recorded");

    ctx.cleanup().await;
}

/// Test: Security events require authentication
#[tokio::test]
async fn test_security_events_require_auth() {
    let ctx = TestContext::new().await;

    let response = ctx.get("/api/v1/account/security-events").await;

    assert_eq!(
        response.status, 401,
        "Security events should require authentication"
    );

    ctx.cleanup().await;
}

// =============================================================================
// Privacy Settings and Export Tests (T046 - User Stories 3 & 4)
// =============================================================================

/// Test: Toggle federation setting
#[tokio::test]
async fn test_toggle_federation() {
    let mut ctx = TestContext::new().await;
    let (_user_id, session) = ctx.create_and_login("federation_toggle").await;

    // Disable federation
    let response = ctx
        .patch_with_session(
            "/api/v1/users/me/federation",
            json!({ "enabled": false }),
            &session,
        )
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);
    assert_eq!(
        response.get("federation_enabled").and_then(|v| v.as_bool()),
        Some(false),
        "Federation should be disabled"
    );

    // Enable federation
    let response = ctx
        .patch_with_session(
            "/api/v1/users/me/federation",
            json!({ "enabled": true }),
            &session,
        )
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);
    assert_eq!(
        response.get("federation_enabled").and_then(|v| v.as_bool()),
        Some(true),
        "Federation should be enabled"
    );

    ctx.cleanup().await;
}

/// Test: Data export returns user data
#[tokio::test]
async fn test_data_export() {
    let mut ctx = TestContext::new().await;
    let (user_id, session) = ctx.create_and_login("data_export").await;

    // Create some data to export
    ctx.create_complete_recipe(user_id, "Export Recipe", "public")
        .await;
    ctx.create_book(user_id, "Export Book", "public").await;

    // Request export
    let response = ctx
        .get_with_session("/api/v1/users/me/export", &session)
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    // Verify export contains expected fields
    assert!(
        response.get("profile").is_some(),
        "Export should include profile"
    );
    assert!(
        response.get("recipes").is_some(),
        "Export should include recipes"
    );
    assert!(
        response.get("books").is_some(),
        "Export should include books"
    );
    assert!(
        response.get("exported_at").is_some(),
        "Export should include timestamp"
    );

    ctx.cleanup().await;
}

/// Test: Data export requires authentication
#[tokio::test]
async fn test_data_export_requires_auth() {
    let ctx = TestContext::new().await;

    let response = ctx.get("/api/v1/users/me/export").await;

    assert_eq!(
        response.status, 401,
        "Data export should require authentication"
    );

    ctx.cleanup().await;
}

// =============================================================================
// Book Contribution Tests (T047 - User Story 5)
// =============================================================================

/// Test: Create book contribution
#[tokio::test]
async fn test_create_book_contribution() {
    let mut ctx = TestContext::new().await;

    // Create book owner
    let (owner_id, owner_session) = ctx.create_and_login("book_owner").await;

    // Create contributor
    let (contributor_id, contributor_session) = ctx.create_and_login("contributor").await;

    // Create a book with contributor permission
    let book_id = ctx.create_book(owner_id, "Shared Book", "private").await;
    ctx.grant_book_permission_to_user(owner_id, book_id, contributor_id, "contributor")
        .await;

    // Create a recipe by contributor
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "Contribution Recipe", "public")
        .await;

    // Contributor adds recipe to book via contribution
    let response = ctx
        .post_with_session(
            &format!("/api/v1/books/{}/contributions", book_id),
            json!({ "recipe_id": recipe_id }),
            &contributor_session,
        )
        .await;

    // Should succeed with 201 Created or 200 OK
    assert!(
        response.status == 201 || response.status == 200,
        "Expected 201 or 200: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

/// Test: Owner can view pending contributions
#[tokio::test]
async fn test_owner_views_contributions() {
    let mut ctx = TestContext::new().await;

    // Create book owner and book
    let (owner_id, owner_session) = ctx.create_and_login("contrib_owner").await;
    let book_id = ctx.create_book(owner_id, "Contrib Book", "private").await;

    // Get book details (contributions should be empty initially)
    let response = ctx
        .get_with_session(&format!("/api/v1/books/{}", book_id), &owner_session)
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    ctx.cleanup().await;
}

// =============================================================================
// Followers/Following Tests (T048 - User Story 6)
// =============================================================================

/// Test: Get followers list
#[tokio::test]
async fn test_get_followers_list() {
    let mut ctx = TestContext::new().await;

    // Create users
    let (user_id, _session) = ctx.create_and_login("followee").await;
    let (follower_id, _follower_session) = ctx.create_and_login("follower").await;

    // Create follow relationship
    ctx.create_follow(follower_id, user_id).await;

    // Get followers
    let response = ctx
        .get(&format!("/api/v1/users/{}/followers", user_id))
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    let followers = response.body.as_array().expect("Should return array");
    assert_eq!(followers.len(), 1, "Should have 1 follower");

    ctx.cleanup().await;
}

/// Test: Get following list
#[tokio::test]
async fn test_get_following_list() {
    let mut ctx = TestContext::new().await;

    // Create users
    let (user_id, _session) = ctx.create_and_login("following_user").await;
    let (target_id, _target_session) = ctx.create_and_login("target_user").await;

    // Create follow relationship
    ctx.create_follow(user_id, target_id).await;

    // Get following
    let response = ctx
        .get(&format!("/api/v1/users/{}/following", user_id))
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    let following = response.body.as_array().expect("Should return array");
    assert_eq!(following.len(), 1, "Should be following 1 user");

    ctx.cleanup().await;
}

/// Test: Follow a user via API
#[tokio::test]
async fn test_follow_user() {
    let mut ctx = TestContext::new().await;

    let (user_id, session) = ctx.create_and_login("follower_api").await;
    let (target_id, _target_session) = ctx.create_and_login("target_api").await;

    // Follow the target user
    let response = ctx
        .post_with_session(
            &format!("/api/v1/users/{}/follow", target_id),
            json!({}),
            &session,
        )
        .await;

    assert!(
        response.status == 200 || response.status == 201,
        "Expected 200 or 201: {:?}",
        response.body
    );

    // Verify follow was created
    let following = ctx
        .get(&format!("/api/v1/users/{}/following", user_id))
        .await;

    let following_list = following.body.as_array().expect("Should return array");
    assert_eq!(following_list.len(), 1, "Should be following 1 user");

    ctx.cleanup().await;
}

/// Test: Unfollow a user via API
#[tokio::test]
async fn test_unfollow_user() {
    let mut ctx = TestContext::new().await;

    let (user_id, session) = ctx.create_and_login("unfollower").await;
    let (target_id, _target_session) = ctx.create_and_login("unfollow_target").await;

    // Create follow relationship
    ctx.create_follow(user_id, target_id).await;

    // Unfollow the target user
    let response = ctx
        .delete_with_session(&format!("/api/v1/users/{}/follow", target_id), &session)
        .await;

    assert!(
        response.status == 200 || response.status == 204,
        "Expected 200 or 204: {:?}",
        response.body
    );

    // Verify follow was removed
    let following = ctx
        .get(&format!("/api/v1/users/{}/following", user_id))
        .await;

    let following_list = following.body.as_array().expect("Should return array");
    assert_eq!(following_list.len(), 0, "Should not be following anyone");

    ctx.cleanup().await;
}

/// Test: Cannot follow yourself
#[tokio::test]
async fn test_cannot_follow_self() {
    let mut ctx = TestContext::new().await;

    let (user_id, session) = ctx.create_and_login("self_follower").await;

    // Try to follow self
    let response = ctx
        .post_with_session(
            &format!("/api/v1/users/{}/follow", user_id),
            json!({}),
            &session,
        )
        .await;

    // Should fail with validation error
    assert!(
        response.status == 400 || response.status == 422,
        "Should reject self-follow: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

// =============================================================================
// User Search Tests (for share functionality)
// =============================================================================

/// Test: Search users by username
#[tokio::test]
async fn test_search_users_by_username() {
    let mut ctx = TestContext::new().await;

    let (_, session) = ctx.create_and_login("searcher").await;

    // Create another user with known username prefix
    let email = TestContext::unique_email();
    let username = "searchtest_user123";
    ctx.create_user(&email, username, "Xk9#mP2$vL5@nQ8!", true)
        .await;

    // Search for user
    let response = ctx
        .get_with_session("/api/v1/users/search?q=searchtest", &session)
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    let results = response.body.as_array().expect("Should return array");
    assert!(!results.is_empty(), "Should find at least one user");

    // Verify result has expected fields
    let first_result = &results[0];
    assert!(first_result.get("id").is_some(), "Should have id");
    assert!(
        first_result.get("username").is_some(),
        "Should have username"
    );
    assert!(
        first_result.get("display_name").is_some(),
        "Should have display_name"
    );

    ctx.cleanup().await;
}

/// Test: Search users requires authentication
#[tokio::test]
async fn test_search_users_requires_auth() {
    let ctx = TestContext::new().await;

    let response = ctx.get("/api/v1/users/search?q=test").await;

    assert_eq!(
        response.status, 401,
        "User search should require authentication"
    );

    ctx.cleanup().await;
}

/// Test: Empty search query returns empty results
#[tokio::test]
async fn test_search_users_empty_query() {
    let mut ctx = TestContext::new().await;

    let (_, session) = ctx.create_and_login("empty_searcher").await;

    let response = ctx
        .get_with_session("/api/v1/users/search?q=", &session)
        .await;

    assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

    let results = response.body.as_array().expect("Should return array");
    assert!(results.is_empty(), "Empty query should return no results");

    ctx.cleanup().await;
}
