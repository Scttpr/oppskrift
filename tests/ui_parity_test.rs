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
    let (owner_id, _owner_session) = ctx.create_and_login("book_owner").await;

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

// =============================================================================
// Authorization Tests - Contribution Accept/Reject (Security Fix Validation)
// =============================================================================

/// Test: Non-owner cannot accept contribution (authorization check)
#[tokio::test]
async fn test_non_owner_cannot_accept_contribution() {
    let mut ctx = TestContext::new().await;

    // Create book owner
    let (owner_id, _owner_session) = ctx.create_and_login("auth_owner").await;

    // Create contributor
    let (contributor_id, _contributor_session) = ctx.create_and_login("auth_contributor").await;

    // Create attacker (neither owner nor contributor)
    let (_attacker_id, attacker_session) = ctx.create_and_login("auth_attacker").await;

    // Create a book and a pending contribution
    let book_id = ctx.create_book(owner_id, "Auth Test Book", "private").await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "Contributed Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // Attacker tries to accept the contribution
    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/accept",
                book_id, contribution_id
            ),
            json!({ "csrf_token": "test" }),
            &attacker_session,
        )
        .await;

    // Should be forbidden (403) or unauthorized
    assert!(
        response.status == 403 || response.status == 401,
        "Non-owner should not be able to accept contribution: status={}, body={:?}",
        response.status,
        response.body
    );

    ctx.cleanup().await;
}

/// Test: Non-owner cannot reject contribution (authorization check)
#[tokio::test]
async fn test_non_owner_cannot_reject_contribution() {
    let mut ctx = TestContext::new().await;

    // Create book owner
    let (owner_id, _owner_session) = ctx.create_and_login("reject_owner").await;

    // Create contributor
    let (contributor_id, _contributor_session) = ctx.create_and_login("reject_contributor").await;

    // Create attacker
    let (_attacker_id, attacker_session) = ctx.create_and_login("reject_attacker").await;

    // Create a book and a pending contribution
    let book_id = ctx
        .create_book(owner_id, "Reject Auth Test Book", "private")
        .await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "Reject Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // Attacker tries to reject the contribution
    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/reject",
                book_id, contribution_id
            ),
            json!({ "csrf_token": "test", "reason": "malicious rejection" }),
            &attacker_session,
        )
        .await;

    // Should be forbidden (403) or unauthorized
    assert!(
        response.status == 403 || response.status == 401,
        "Non-owner should not be able to reject contribution: status={}, body={:?}",
        response.status,
        response.body
    );

    ctx.cleanup().await;
}

// =============================================================================
// Contribution Accept/Reject Flow Tests
// =============================================================================

/// Test: Owner can accept a pending contribution
#[tokio::test]
async fn test_owner_can_accept_contribution() {
    let mut ctx = TestContext::new().await;

    // Create book owner
    let (owner_id, owner_session) = ctx.create_and_login("accept_owner").await;

    // Create contributor
    let (contributor_id, _contributor_session) = ctx.create_and_login("accept_contrib").await;

    // Create book, recipe, and contribution
    let book_id = ctx
        .create_book(owner_id, "Accept Test Book", "private")
        .await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "Accept Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // Owner accepts the contribution
    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/accept",
                book_id, contribution_id
            ),
            json!({ "csrf_token": "test" }),
            &owner_session,
        )
        .await;

    // Should succeed (200 or redirect)
    assert!(
        response.status == 200 || response.status == 302 || response.status == 303,
        "Owner should be able to accept contribution: status={}, body={:?}",
        response.status,
        response.body
    );

    // Verify contribution status changed to accepted
    let status: Option<String> =
        sqlx::query_scalar("SELECT status::text FROM book_contributions WHERE id = $1")
            .bind(contribution_id)
            .fetch_optional(&ctx.db)
            .await
            .expect("Failed to query contribution");

    assert_eq!(
        status,
        Some("accepted".to_string()),
        "Contribution status should be 'accepted'"
    );

    ctx.cleanup().await;
}

/// Test: Owner can reject a pending contribution
#[tokio::test]
async fn test_owner_can_reject_contribution() {
    let mut ctx = TestContext::new().await;

    // Create book owner
    let (owner_id, owner_session) = ctx.create_and_login("reject_own").await;

    // Create contributor
    let (contributor_id, _contributor_session) = ctx.create_and_login("reject_contrib").await;

    // Create book, recipe, and contribution
    let book_id = ctx
        .create_book(owner_id, "Reject Test Book", "private")
        .await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "Reject Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // Owner rejects the contribution
    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/reject",
                book_id, contribution_id
            ),
            json!({ "csrf_token": "test", "reason": "Not a good fit" }),
            &owner_session,
        )
        .await;

    // Should succeed (200 or redirect)
    assert!(
        response.status == 200 || response.status == 302 || response.status == 303,
        "Owner should be able to reject contribution: status={}, body={:?}",
        response.status,
        response.body
    );

    // Verify contribution status changed to rejected
    let status: Option<String> =
        sqlx::query_scalar("SELECT status::text FROM book_contributions WHERE id = $1")
            .bind(contribution_id)
            .fetch_optional(&ctx.db)
            .await
            .expect("Failed to query contribution");

    assert_eq!(
        status,
        Some("rejected".to_string()),
        "Contribution status should be 'rejected'"
    );

    ctx.cleanup().await;
}

// =============================================================================
// Individual Session Revoke Tests
// =============================================================================

/// Test: Revoke a specific session by ID
#[tokio::test]
async fn test_revoke_individual_session() {
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

    // Get session1's ID
    let session1_id = ctx
        .get_session_id(&session1)
        .await
        .expect("Should find session1 ID");

    // From session2, revoke session1 specifically
    let response = ctx
        .delete_with_session(
            &format!("/api/v1/account/sessions/{}", session1_id),
            &session2,
        )
        .await;

    assert!(
        response.status == 200 || response.status == 204,
        "Should successfully revoke session: {:?}",
        response.body
    );

    // session1 should now be invalid
    let check1 = ctx
        .get_with_session("/api/v1/account/profile", &session1)
        .await;
    assert_eq!(check1.status, 401, "Revoked session should be invalid");

    // session2 should still work
    let check2 = ctx
        .get_with_session("/api/v1/account/profile", &session2)
        .await;
    assert_eq!(check2.status, 200, "Current session should still be valid");

    ctx.cleanup().await;
}

/// Test: Cannot revoke your own current session via individual revoke
#[tokio::test]
async fn test_cannot_revoke_current_session() {
    let mut ctx = TestContext::new().await;
    let (_user_id, session) = ctx.create_and_login("self_revoke").await;

    // Get current session's ID
    let session_id = ctx
        .get_session_id(&session)
        .await
        .expect("Should find session ID");

    // Try to revoke own session
    let response = ctx
        .delete_with_session(
            &format!("/api/v1/account/sessions/{}", session_id),
            &session,
        )
        .await;

    // Should fail with 400 or 403 (can't revoke current session)
    assert!(
        response.status == 400 || response.status == 403,
        "Should not be able to revoke current session: status={}, body={:?}",
        response.status,
        response.body
    );

    ctx.cleanup().await;
}

// =============================================================================
// Export Rate Limit Tests
// =============================================================================

/// Test: Export rate limit is enforced
#[tokio::test]
async fn test_export_rate_limit() {
    let mut ctx = TestContext::new().await;
    let (user_id, session) = ctx.create_and_login("rate_limit").await;

    // First export should succeed
    let response1 = ctx
        .get_with_session("/api/v1/users/me/export", &session)
        .await;

    assert_eq!(
        response1.status, 200,
        "First export should succeed: {:?}",
        response1.body
    );

    // Insert a fake export event within the last hour
    sqlx::query(
        r#"
        INSERT INTO security_events (user_id, event_type, ip_address, user_agent)
        VALUES ($1, 'account_export', '127.0.0.1', 'test')
        "#,
    )
    .bind(user_id)
    .execute(&ctx.db)
    .await
    .expect("Failed to insert security event");

    // Second export should be rate limited
    let response2 = ctx
        .get_with_session("/api/v1/users/me/export", &session)
        .await;

    assert_eq!(
        response2.status, 400,
        "Second export should be rate limited: {:?}",
        response2.body
    );

    // Error message should mention rate limit
    let error_msg = response2.error_message().unwrap_or("");
    assert!(
        error_msg.contains("rate") || error_msg.contains("limit") || error_msg.contains("wait"),
        "Error should mention rate limit: {}",
        error_msg
    );

    ctx.cleanup().await;
}

// =============================================================================
// Pagination Tests for Followers/Following
// =============================================================================

/// Test: Followers pagination works correctly
#[tokio::test]
async fn test_followers_pagination() {
    let mut ctx = TestContext::new().await;

    // Create target user
    let (target_id, _target_session) = ctx.create_and_login("paginate_target").await;

    // Create 25 followers (more than page size of 20)
    for i in 0..25 {
        let (follower_id, _) = ctx
            .create_and_login(&format!("paginate_follower_{}", i))
            .await;
        ctx.create_follow(follower_id, target_id).await;
    }

    // Get first page
    let response1 = ctx
        .get(&format!("/api/v1/users/{}/followers?page=1", target_id))
        .await;

    assert_eq!(
        response1.status, 200,
        "First page should succeed: {:?}",
        response1.body
    );

    let followers1 = response1.body.as_array().expect("Should return array");
    assert_eq!(
        followers1.len(),
        20,
        "First page should have 20 followers (page size)"
    );

    // Get second page
    let response2 = ctx
        .get(&format!("/api/v1/users/{}/followers?page=2", target_id))
        .await;

    assert_eq!(
        response2.status, 200,
        "Second page should succeed: {:?}",
        response2.body
    );

    let followers2 = response2.body.as_array().expect("Should return array");
    assert_eq!(
        followers2.len(),
        5,
        "Second page should have remaining 5 followers"
    );

    ctx.cleanup().await;
}

/// Test: Following pagination works correctly
#[tokio::test]
async fn test_following_pagination() {
    let mut ctx = TestContext::new().await;

    // Create user who follows many
    let (user_id, _user_session) = ctx.create_and_login("following_user").await;

    // Create 25 users to follow
    for i in 0..25 {
        let (target_id, _) = ctx
            .create_and_login(&format!("following_target_{}", i))
            .await;
        ctx.create_follow(user_id, target_id).await;
    }

    // Get first page
    let response1 = ctx
        .get(&format!("/api/v1/users/{}/following?page=1", user_id))
        .await;

    assert_eq!(
        response1.status, 200,
        "First page should succeed: {:?}",
        response1.body
    );

    let following1 = response1.body.as_array().expect("Should return array");
    assert_eq!(
        following1.len(),
        20,
        "First page should have 20 users (page size)"
    );

    // Get second page
    let response2 = ctx
        .get(&format!("/api/v1/users/{}/following?page=2", user_id))
        .await;

    assert_eq!(
        response2.status, 200,
        "Second page should succeed: {:?}",
        response2.body
    );

    let following2 = response2.body.as_array().expect("Should return array");
    assert_eq!(
        following2.len(),
        5,
        "Second page should have remaining 5 users"
    );

    ctx.cleanup().await;
}

// =============================================================================
// Error Path Tests - Invalid IDs and Edge Cases
// =============================================================================

/// Test: Revoke non-existent session returns 404
#[tokio::test]
async fn test_revoke_invalid_session_id() {
    let mut ctx = TestContext::new().await;
    let (_user_id, session) = ctx.create_and_login("invalid_revoke").await;

    // Try to revoke a random UUID that doesn't exist
    let fake_session_id = uuid::Uuid::new_v4();
    let response = ctx
        .delete_with_session(
            &format!("/api/v1/account/sessions/{}", fake_session_id),
            &session,
        )
        .await;

    assert_eq!(
        response.status, 404,
        "Revoking non-existent session should return 404: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

/// Test: Get followers for non-existent user returns 404
#[tokio::test]
async fn test_get_followers_invalid_user() {
    let ctx = TestContext::new().await;

    let fake_user_id = uuid::Uuid::new_v4();
    let response = ctx
        .get(&format!("/api/v1/users/{}/followers", fake_user_id))
        .await;

    assert_eq!(
        response.status, 404,
        "Followers for non-existent user should return 404: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

/// Test: Get following for non-existent user returns 404
#[tokio::test]
async fn test_get_following_invalid_user() {
    let ctx = TestContext::new().await;

    let fake_user_id = uuid::Uuid::new_v4();
    let response = ctx
        .get(&format!("/api/v1/users/{}/following", fake_user_id))
        .await;

    assert_eq!(
        response.status, 404,
        "Following for non-existent user should return 404: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

/// Test: Follow non-existent user returns 404
#[tokio::test]
async fn test_follow_nonexistent_user() {
    let mut ctx = TestContext::new().await;
    let (_user_id, session) = ctx.create_and_login("follower_404").await;

    let fake_user_id = uuid::Uuid::new_v4();
    let response = ctx
        .post_with_session(
            &format!("/api/v1/users/{}/follow", fake_user_id),
            json!({}),
            &session,
        )
        .await;

    assert_eq!(
        response.status, 404,
        "Following non-existent user should return 404: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

// =============================================================================
// Input Validation and Security Tests
// =============================================================================

/// Test: User search sanitizes special characters
#[tokio::test]
async fn test_search_special_characters() {
    let mut ctx = TestContext::new().await;
    let (_, session) = ctx.create_and_login("search_special").await;

    // Test SQL injection attempt
    let response = ctx
        .get_with_session("/api/v1/users/search?q='; DROP TABLE users;--", &session)
        .await;

    assert_eq!(
        response.status, 200,
        "SQL injection attempt should not crash: {:?}",
        response.body
    );

    // Results should be empty (special chars filtered)
    let results = response.body.as_array().expect("Should return array");
    assert!(
        results.is_empty(),
        "SQL injection query should return empty results"
    );

    ctx.cleanup().await;
}

/// Test: User search with regex special characters
#[tokio::test]
async fn test_search_regex_characters() {
    let mut ctx = TestContext::new().await;
    let (_, session) = ctx.create_and_login("search_regex").await;

    // Test regex special characters
    let response = ctx
        .get_with_session("/api/v1/users/search?q=.*%25", &session)
        .await;

    assert_eq!(
        response.status, 200,
        "Regex characters should not crash: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

/// Test: User search with very long query
#[tokio::test]
async fn test_search_long_query() {
    let mut ctx = TestContext::new().await;
    let (_, session) = ctx.create_and_login("search_long").await;

    // Create a very long search query (1000 chars)
    let long_query = "a".repeat(1000);
    let response = ctx
        .get_with_session(&format!("/api/v1/users/search?q={}", long_query), &session)
        .await;

    // Should handle gracefully (200 with empty results, not 500)
    assert!(
        response.status == 200 || response.status == 400,
        "Long query should be handled gracefully: status={}, body={:?}",
        response.status,
        response.body
    );

    ctx.cleanup().await;
}

/// Test: Search limit parameter is clamped
#[tokio::test]
async fn test_search_limit_clamping() {
    let mut ctx = TestContext::new().await;
    let (_, session) = ctx.create_and_login("search_limit").await;

    // Create some users to find
    for i in 0..5 {
        let email = TestContext::unique_email();
        ctx.create_user(&email, &format!("limituser{}", i), "Xk9#mP2$vL5@nQ8!", true)
            .await;
    }

    // Test limit > 50 (should clamp to 50)
    let response = ctx
        .get_with_session("/api/v1/users/search?q=limituser&limit=100", &session)
        .await;

    assert_eq!(response.status, 200, "Should succeed: {:?}", response.body);

    // Test limit = 0 (should clamp to 1 or return empty)
    let response = ctx
        .get_with_session("/api/v1/users/search?q=limituser&limit=0", &session)
        .await;

    assert_eq!(
        response.status, 200,
        "Zero limit should be handled: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

// =============================================================================
// Idempotency and State Tests
// =============================================================================

/// Test: Double-accept contribution (idempotency)
#[tokio::test]
async fn test_double_accept_contribution() {
    let mut ctx = TestContext::new().await;

    let (owner_id, owner_session) = ctx.create_and_login("double_owner").await;
    let (contributor_id, _) = ctx.create_and_login("double_contrib").await;

    let book_id = ctx
        .create_book(owner_id, "Double Accept Book", "private")
        .await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "Double Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // First accept
    let response1 = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/accept",
                book_id, contribution_id
            ),
            json!({ "csrf_token": "test" }),
            &owner_session,
        )
        .await;

    assert!(
        response1.status == 200 || response1.status == 302,
        "First accept should succeed: {:?}",
        response1.body
    );

    // Second accept (already accepted)
    let response2 = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/accept",
                book_id, contribution_id
            ),
            json!({ "csrf_token": "test" }),
            &owner_session,
        )
        .await;

    // Should either succeed (idempotent) or return 400/409 (already processed)
    assert!(
        response2.status == 200
            || response2.status == 302
            || response2.status == 400
            || response2.status == 409,
        "Double accept should be handled gracefully: status={}, body={:?}",
        response2.status,
        response2.body
    );

    ctx.cleanup().await;
}

/// Test: Accept contribution for non-existent book
#[tokio::test]
async fn test_accept_contribution_invalid_book() {
    let mut ctx = TestContext::new().await;
    let (_user_id, session) = ctx.create_and_login("invalid_book").await;

    let fake_book_id = uuid::Uuid::new_v4();
    let fake_contribution_id = uuid::Uuid::new_v4();

    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/accept",
                fake_book_id, fake_contribution_id
            ),
            json!({ "csrf_token": "test" }),
            &session,
        )
        .await;

    assert_eq!(
        response.status, 404,
        "Accept on non-existent book should return 404: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

// =============================================================================
// Export Threshold Tests
// =============================================================================

/// Test: Export with many recipes (threshold behavior)
#[tokio::test]
async fn test_export_many_recipes_threshold() {
    let mut ctx = TestContext::new().await;
    let (user_id, session) = ctx.create_and_login("many_recipes").await;

    // Create 51 recipes (just over the threshold)
    for i in 0..51 {
        ctx.create_recipe(user_id, &format!("Recipe {}", i), "public")
            .await;
    }

    // Export should fail with >50 recipes
    let response = ctx
        .get_with_session("/api/v1/users/me/export", &session)
        .await;

    assert_eq!(
        response.status, 400,
        "Export with >50 recipes should be rejected: {:?}",
        response.body
    );

    // Error should mention async or large export
    let error_msg = response.error_message().unwrap_or("");
    assert!(
        error_msg.contains("Large") || error_msg.contains("async") || error_msg.contains("50"),
        "Error should mention large export threshold: {}",
        error_msg
    );

    ctx.cleanup().await;
}

/// Test: Export with exactly 50 recipes succeeds
#[tokio::test]
async fn test_export_exactly_50_recipes() {
    let mut ctx = TestContext::new().await;
    let (user_id, session) = ctx.create_and_login("fifty_recipes").await;

    // Create exactly 50 recipes (at the threshold)
    for i in 0..50 {
        ctx.create_recipe(user_id, &format!("Recipe {}", i), "public")
            .await;
    }

    // Export should succeed with exactly 50 recipes
    let response = ctx
        .get_with_session("/api/v1/users/me/export", &session)
        .await;

    assert_eq!(
        response.status, 200,
        "Export with exactly 50 recipes should succeed: {:?}",
        response.body
    );

    ctx.cleanup().await;
}

// =============================================================================
// CSRF Validation Tests
// =============================================================================

/// Test: Accept contribution without CSRF token fails
#[tokio::test]
async fn test_accept_contribution_missing_csrf() {
    let mut ctx = TestContext::new().await;

    let (owner_id, owner_session) = ctx.create_and_login("csrf_owner").await;
    let (contributor_id, _) = ctx.create_and_login("csrf_contrib").await;

    let book_id = ctx.create_book(owner_id, "CSRF Test Book", "private").await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "CSRF Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // Try to accept without CSRF token
    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/accept",
                book_id, contribution_id
            ),
            json!({}), // No csrf_token
            &owner_session,
        )
        .await;

    // Should fail with 400 or 403
    assert!(
        response.status == 400 || response.status == 403 || response.status == 422,
        "Missing CSRF should be rejected: status={}, body={:?}",
        response.status,
        response.body
    );

    ctx.cleanup().await;
}

/// Test: Accept contribution with invalid CSRF token fails
#[tokio::test]
async fn test_accept_contribution_invalid_csrf() {
    let mut ctx = TestContext::new().await;

    let (owner_id, owner_session) = ctx.create_and_login("csrf_invalid_owner").await;
    let (contributor_id, _) = ctx.create_and_login("csrf_invalid_contrib").await;

    let book_id = ctx
        .create_book(owner_id, "CSRF Invalid Book", "private")
        .await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "CSRF Invalid Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // Try to accept with invalid CSRF token
    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/accept",
                book_id, contribution_id
            ),
            json!({ "csrf_token": "invalid_token_12345" }),
            &owner_session,
        )
        .await;

    // Should fail with 400 or 403
    assert!(
        response.status == 400 || response.status == 403,
        "Invalid CSRF should be rejected: status={}, body={:?}",
        response.status,
        response.body
    );

    ctx.cleanup().await;
}

/// Test: Reject contribution without CSRF token fails
#[tokio::test]
async fn test_reject_contribution_missing_csrf() {
    let mut ctx = TestContext::new().await;

    let (owner_id, owner_session) = ctx.create_and_login("csrf_reject_owner").await;
    let (contributor_id, _) = ctx.create_and_login("csrf_reject_contrib").await;

    let book_id = ctx
        .create_book(owner_id, "CSRF Reject Book", "private")
        .await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "CSRF Reject Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // Try to reject without CSRF token
    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/reject",
                book_id, contribution_id
            ),
            json!({ "reason": "test" }), // No csrf_token
            &owner_session,
        )
        .await;

    // Should fail with 400 or 403
    assert!(
        response.status == 400 || response.status == 403 || response.status == 422,
        "Missing CSRF should be rejected: status={}, body={:?}",
        response.status,
        response.body
    );

    ctx.cleanup().await;
}

// =============================================================================
// Data Consistency Tests
// =============================================================================

/// Test: After accepting contribution, recipe appears in book
#[tokio::test]
async fn test_accepted_contribution_adds_recipe_to_book() {
    let mut ctx = TestContext::new().await;

    let (owner_id, owner_session) = ctx.create_and_login("consist_owner").await;
    let (contributor_id, _) = ctx.create_and_login("consist_contrib").await;

    let book_id = ctx
        .create_book(owner_id, "Consistency Book", "public")
        .await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "Consistency Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // Verify recipe is NOT in book before accept
    let before_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM book_recipe_entries WHERE book_id = $1 AND recipe_id = $2",
    )
    .bind(book_id)
    .bind(recipe_id)
    .fetch_one(&ctx.db)
    .await
    .expect("Failed to count entries");

    assert_eq!(
        before_count, 0,
        "Recipe should not be in book before accept"
    );

    // Accept the contribution
    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/accept",
                book_id, contribution_id
            ),
            json!({ "csrf_token": "test" }),
            &owner_session,
        )
        .await;

    assert!(
        response.status == 200 || response.status == 302,
        "Accept should succeed: {:?}",
        response.body
    );

    // Verify recipe IS in book after accept
    let after_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM book_recipe_entries WHERE book_id = $1 AND recipe_id = $2",
    )
    .bind(book_id)
    .bind(recipe_id)
    .fetch_one(&ctx.db)
    .await
    .expect("Failed to count entries");

    assert_eq!(after_count, 1, "Recipe should be in book after accept");

    ctx.cleanup().await;
}

/// Test: After rejecting contribution, recipe does NOT appear in book
#[tokio::test]
async fn test_rejected_contribution_does_not_add_recipe() {
    let mut ctx = TestContext::new().await;

    let (owner_id, owner_session) = ctx.create_and_login("reject_consist_owner").await;
    let (contributor_id, _) = ctx.create_and_login("reject_consist_contrib").await;

    let book_id = ctx
        .create_book(owner_id, "Reject Consistency Book", "public")
        .await;
    let recipe_id = ctx
        .create_complete_recipe(contributor_id, "Reject Consistency Recipe", "public")
        .await;
    let contribution_id = ctx
        .create_book_contribution(book_id, recipe_id, contributor_id)
        .await;

    // Reject the contribution
    let response = ctx
        .post_with_session(
            &format!(
                "/books/{}/contributions/{}/reject",
                book_id, contribution_id
            ),
            json!({ "csrf_token": "test", "reason": "Not suitable" }),
            &owner_session,
        )
        .await;

    assert!(
        response.status == 200 || response.status == 302,
        "Reject should succeed: {:?}",
        response.body
    );

    // Verify recipe is NOT in book
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM book_recipe_entries WHERE book_id = $1 AND recipe_id = $2",
    )
    .bind(book_id)
    .bind(recipe_id)
    .fetch_one(&ctx.db)
    .await
    .expect("Failed to count entries");

    assert_eq!(count, 0, "Recipe should NOT be in book after reject");

    ctx.cleanup().await;
}

/// Test: Follow count increases after following
#[tokio::test]
async fn test_follow_increases_count() {
    let mut ctx = TestContext::new().await;

    let (_follower_id, follower_session) = ctx.create_and_login("count_follower").await;
    let (target_id, _) = ctx.create_and_login("count_target").await;

    // Get initial follower count
    let initial_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM follows WHERE following_id = $1")
            .bind(target_id)
            .fetch_one(&ctx.db)
            .await
            .expect("Failed to count followers");

    assert_eq!(initial_count, 0, "Should start with 0 followers");

    // Follow the target
    let response = ctx
        .post_with_session(
            &format!("/api/v1/users/{}/follow", target_id),
            json!({}),
            &follower_session,
        )
        .await;

    assert!(
        response.status == 200 || response.status == 201,
        "Follow should succeed: {:?}",
        response.body
    );

    // Check follower count increased
    let after_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM follows WHERE following_id = $1")
            .bind(target_id)
            .fetch_one(&ctx.db)
            .await
            .expect("Failed to count followers");

    assert_eq!(after_count, 1, "Should have 1 follower after follow");

    ctx.cleanup().await;
}

/// Test: Follow count decreases after unfollowing
#[tokio::test]
async fn test_unfollow_decreases_count() {
    let mut ctx = TestContext::new().await;

    let (follower_id, follower_session) = ctx.create_and_login("uncount_follower").await;
    let (target_id, _) = ctx.create_and_login("uncount_target").await;

    // Create follow relationship
    ctx.create_follow(follower_id, target_id).await;

    // Verify we have 1 follower
    let initial_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM follows WHERE following_id = $1")
            .bind(target_id)
            .fetch_one(&ctx.db)
            .await
            .expect("Failed to count followers");

    assert_eq!(initial_count, 1, "Should have 1 follower");

    // Unfollow
    let response = ctx
        .delete_with_session(
            &format!("/api/v1/users/{}/follow", target_id),
            &follower_session,
        )
        .await;

    assert!(
        response.status == 200 || response.status == 204,
        "Unfollow should succeed: {:?}",
        response.body
    );

    // Check follower count decreased
    let after_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM follows WHERE following_id = $1")
            .bind(target_id)
            .fetch_one(&ctx.db)
            .await
            .expect("Failed to count followers");

    assert_eq!(after_count, 0, "Should have 0 followers after unfollow");

    ctx.cleanup().await;
}

// =============================================================================
// Idempotency Tests - Follow/Unfollow
// =============================================================================

/// Test: Double follow is handled gracefully
#[tokio::test]
async fn test_double_follow() {
    let mut ctx = TestContext::new().await;

    let (_follower_id, follower_session) = ctx.create_and_login("double_follower").await;
    let (target_id, _) = ctx.create_and_login("double_target").await;

    // First follow
    let response1 = ctx
        .post_with_session(
            &format!("/api/v1/users/{}/follow", target_id),
            json!({}),
            &follower_session,
        )
        .await;

    assert!(
        response1.status == 200 || response1.status == 201,
        "First follow should succeed: {:?}",
        response1.body
    );

    // Second follow (already following)
    let response2 = ctx
        .post_with_session(
            &format!("/api/v1/users/{}/follow", target_id),
            json!({}),
            &follower_session,
        )
        .await;

    // Should either succeed (idempotent) or return 409 (conflict)
    assert!(
        response2.status == 200 || response2.status == 201 || response2.status == 409,
        "Double follow should be handled gracefully: status={}, body={:?}",
        response2.status,
        response2.body
    );

    // Verify still only 1 follow relationship
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM follows WHERE following_id = $1")
        .bind(target_id)
        .fetch_one(&ctx.db)
        .await
        .expect("Failed to count followers");

    assert_eq!(count, 1, "Should still have only 1 follower");

    ctx.cleanup().await;
}

/// Test: Double unfollow is handled gracefully
#[tokio::test]
async fn test_double_unfollow() {
    let mut ctx = TestContext::new().await;

    let (follower_id, follower_session) = ctx.create_and_login("double_unfollower").await;
    let (target_id, _) = ctx.create_and_login("double_unfollow_target").await;

    // Create follow relationship
    ctx.create_follow(follower_id, target_id).await;

    // First unfollow
    let response1 = ctx
        .delete_with_session(
            &format!("/api/v1/users/{}/follow", target_id),
            &follower_session,
        )
        .await;

    assert!(
        response1.status == 200 || response1.status == 204,
        "First unfollow should succeed: {:?}",
        response1.body
    );

    // Second unfollow (already unfollowed)
    let response2 = ctx
        .delete_with_session(
            &format!("/api/v1/users/{}/follow", target_id),
            &follower_session,
        )
        .await;

    // Should either succeed (idempotent) or return 404 (not found)
    assert!(
        response2.status == 200 || response2.status == 204 || response2.status == 404,
        "Double unfollow should be handled gracefully: status={}, body={:?}",
        response2.status,
        response2.body
    );

    ctx.cleanup().await;
}

/// Test: Security event recorded on session revoke
#[tokio::test]
async fn test_security_event_on_session_revoke() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let password = "Xk9#mP2$vL5@nQ8!";

    let user_id = ctx.create_user(&email, &username, password, true).await;

    // Create two sessions
    let session1 = ctx
        .login_and_get_session(&email, password)
        .await
        .expect("First login should succeed");

    let session2 = ctx
        .login_and_get_session(&email, password)
        .await
        .expect("Second login should succeed");

    // Get initial security event count
    let initial_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM security_events WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx.db)
            .await
            .expect("Failed to count events");

    // Revoke session1 from session2
    let session1_id = ctx
        .get_session_id(&session1)
        .await
        .expect("Should find session ID");
    let response = ctx
        .delete_with_session(
            &format!("/api/v1/account/sessions/{}", session1_id),
            &session2,
        )
        .await;

    assert!(
        response.status == 200 || response.status == 204,
        "Revoke should succeed: {:?}",
        response.body
    );

    // Check security event was recorded
    let after_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM security_events WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&ctx.db)
            .await
            .expect("Failed to count events");

    assert!(
        after_count > initial_count,
        "Security event should be recorded: before={}, after={}",
        initial_count,
        after_count
    );

    ctx.cleanup().await;
}
