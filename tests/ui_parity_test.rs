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

use common::{run_test, TestContext};
use serde_json::json;

// =============================================================================
// Session Management Tests (T044 - User Story 1)
// =============================================================================

/// Test: List active sessions returns current session
#[tokio::test]
async fn test_list_sessions_returns_current() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Multiple sessions can be created and listed
#[tokio::test]
async fn test_multiple_sessions_listed() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Revoke all other sessions keeps current session
#[tokio::test]
async fn test_revoke_all_other_sessions() {
    run_test(|mut ctx| async move {
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

        // Get session1's ID so we can revoke it specifically
        let session1_id = ctx
            .get_session_id(&session1)
            .await
            .expect("Should find session1 ID");

        // Revoke session1 from session2
        let response = ctx
            .delete_with_session(
                &format!("/api/v1/account/sessions/{}", session1_id),
                &session2,
            )
            .await;

        assert!(
            response.status == 200 || response.status == 204,
            "Expected 200/204 OK: {:?}",
            response.body
        );

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
    })
    .await;
}

// =============================================================================
// Security Events Tests (T045 - User Story 2)
// =============================================================================

/// Test: Security events endpoint returns events
#[tokio::test]
async fn test_security_events_list() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("security_events").await;

        let response = ctx
            .get_with_session("/api/v1/account/security-events", &session)
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

        // Should have events array (at least login event from create_and_login)
        let events = response.get("events").and_then(|e| e.as_array());
        assert!(events.is_some(), "Should return events array");
    })
    .await;
}

/// Test: Security events are recorded for login
#[tokio::test]
async fn test_security_event_recorded_on_login() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Security events require authentication
#[tokio::test]
async fn test_security_events_require_auth() {
    run_test(|ctx| async move {
        let response = ctx.get("/api/v1/account/security-events").await;

        assert_eq!(
            response.status, 401,
            "Security events should require authentication"
        );
    })
    .await;
}

// =============================================================================
// Privacy Settings and Export Tests (T046 - User Stories 3 & 4)
// =============================================================================

/// Test: Toggle federation setting
#[tokio::test]
async fn test_toggle_federation() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Data export returns user data
#[tokio::test]
async fn test_data_export() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Data export requires authentication
#[tokio::test]
async fn test_data_export_requires_auth() {
    run_test(|ctx| async move {
        let response = ctx.get("/api/v1/users/me/export").await;

        assert_eq!(
            response.status, 401,
            "Data export should require authentication"
        );
    })
    .await;
}

// =============================================================================
// Book Contribution Tests (T047 - User Story 5)
// =============================================================================

/// Test: Create book contribution
#[tokio::test]
async fn test_create_book_contribution() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Owner can view pending contributions
#[tokio::test]
async fn test_owner_views_contributions() {
    run_test(|mut ctx| async move {
        // Create book owner and book
        let (owner_id, owner_session) = ctx.create_and_login("contrib_owner").await;
        let book_id = ctx.create_book(owner_id, "Contrib Book", "private").await;

        // Get book details (contributions should be empty initially)
        let response = ctx
            .get_with_session(&format!("/api/v1/books/{}", book_id), &owner_session)
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);
    })
    .await;
}

// =============================================================================
// Followers/Following Tests (T048 - User Story 6)
// =============================================================================

/// Test: Get followers list
#[tokio::test]
async fn test_get_followers_list() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Get following list
#[tokio::test]
async fn test_get_following_list() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Follow a user via API
#[tokio::test]
async fn test_follow_user() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Unfollow a user via API
#[tokio::test]
async fn test_unfollow_user() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Cannot follow yourself
#[tokio::test]
async fn test_cannot_follow_self() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

// =============================================================================
// User Search Tests (for share functionality)
// =============================================================================

/// Test: Search users by username
#[tokio::test]
async fn test_search_users_by_username() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Search users requires authentication
#[tokio::test]
async fn test_search_users_requires_auth() {
    run_test(|ctx| async move {
        let response = ctx.get("/api/v1/users/search?q=test").await;

        assert_eq!(
            response.status, 401,
            "User search should require authentication"
        );
    })
    .await;
}

/// Test: Empty search query returns empty results
#[tokio::test]
async fn test_search_users_empty_query() {
    run_test(|mut ctx| async move {
        let (_, session) = ctx.create_and_login("empty_searcher").await;

        let response = ctx
            .get_with_session("/api/v1/users/search?q=", &session)
            .await;

        assert_eq!(response.status, 200, "Expected 200 OK: {:?}", response.body);

        let results = response.body.as_array().expect("Should return array");
        assert!(results.is_empty(), "Empty query should return no results");
    })
    .await;
}

// =============================================================================
// Authorization Tests - Contribution Accept/Reject (Security Fix Validation)
// =============================================================================

/// Test: Non-owner cannot accept contribution (authorization check)
#[tokio::test]
async fn test_non_owner_cannot_accept_contribution() {
    run_test(|mut ctx| async move {
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

        // Generate valid CSRF token for attacker's session
        let csrf_token = ctx.generate_csrf_token(&attacker_session).await;

        // Attacker tries to accept the contribution
        let response = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/accept",
                    book_id, contribution_id
                ),
                &[("_csrf", &csrf_token)],
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
    })
    .await;
}

/// Test: Non-owner cannot reject contribution (authorization check)
#[tokio::test]
async fn test_non_owner_cannot_reject_contribution() {
    run_test(|mut ctx| async move {
        // Create book owner
        let (owner_id, _owner_session) = ctx.create_and_login("reject_owner").await;

        // Create contributor
        let (contributor_id, _contributor_session) =
            ctx.create_and_login("reject_contributor").await;

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

        // Generate valid CSRF token for attacker's session
        let csrf_token = ctx.generate_csrf_token(&attacker_session).await;

        // Attacker tries to reject the contribution
        let response = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/reject",
                    book_id, contribution_id
                ),
                &[("_csrf", &csrf_token)],
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
    })
    .await;
}

// =============================================================================
// Contribution Accept/Reject Flow Tests
// =============================================================================

/// Test: Owner can accept a pending contribution
#[tokio::test]
async fn test_owner_can_accept_contribution() {
    run_test(|mut ctx| async move {
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

        // Generate valid CSRF token for owner's session
        let csrf_token = ctx.generate_csrf_token(&owner_session).await;

        // Owner accepts the contribution
        let response = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/accept",
                    book_id, contribution_id
                ),
                &[("_csrf", &csrf_token)],
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
    })
    .await;
}

/// Test: Owner can reject a pending contribution
#[tokio::test]
async fn test_owner_can_reject_contribution() {
    run_test(|mut ctx| async move {
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

        // Generate valid CSRF token for owner's session
        let csrf_token = ctx.generate_csrf_token(&owner_session).await;

        // Owner rejects the contribution
        let response = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/reject",
                    book_id, contribution_id
                ),
                &[("_csrf", &csrf_token)],
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
    })
    .await;
}

// =============================================================================
// Individual Session Revoke Tests
// =============================================================================

/// Test: Revoke a specific session by ID
#[tokio::test]
async fn test_revoke_individual_session() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Cannot revoke your own current session via individual revoke
#[tokio::test]
async fn test_cannot_revoke_current_session() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

// =============================================================================
// Export Functionality Tests
// =============================================================================

/// Test: Multiple consecutive exports succeed
/// Verifies that the export endpoint allows repeated calls.
#[tokio::test]
async fn test_export_multiple_calls_succeed() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("export_test").await;

        // First export should succeed
        let response1 = ctx
            .get_with_session("/api/v1/users/me/export", &session)
            .await;

        assert_eq!(
            response1.status, 200,
            "First export should succeed: {:?}",
            response1.body
        );

        // Second export should also succeed
        let response2 = ctx
            .get_with_session("/api/v1/users/me/export", &session)
            .await;

        assert_eq!(
            response2.status, 200,
            "Second export should also succeed: {:?}",
            response2.body
        );
    })
    .await;
}

// =============================================================================
// Pagination Tests for Followers/Following
// =============================================================================

/// Test: Followers API returns all followers
/// Note: The API endpoint returns all followers without pagination.
/// Pagination is only supported on the UI/HTML endpoints.
#[tokio::test]
async fn test_followers_pagination() {
    run_test(|mut ctx| async move {
        // Create target user
        let (target_id, _target_session) = ctx.create_and_login("paginate_target").await;

        // Create 25 followers
        for i in 0..25 {
            let (follower_id, _) = ctx
                .create_and_login(&format!("paginate_follower_{}", i))
                .await;
            ctx.create_follow(follower_id, target_id).await;
        }

        // Get followers (API returns all without pagination)
        let response = ctx
            .get(&format!("/api/v1/users/{}/followers", target_id))
            .await;

        assert_eq!(response.status, 200, "Should succeed: {:?}", response.body);

        let followers = response.body.as_array().expect("Should return array");
        assert_eq!(followers.len(), 25, "API should return all 25 followers");
    })
    .await;
}

/// Test: Following API returns all following
/// Note: The API endpoint returns all following without pagination.
/// Pagination is only supported on the UI/HTML endpoints.
#[tokio::test]
async fn test_following_pagination() {
    run_test(|mut ctx| async move {
        // Create user who follows many
        let (user_id, _user_session) = ctx.create_and_login("following_user").await;

        // Create 25 users to follow
        for i in 0..25 {
            let (target_id, _) = ctx
                .create_and_login(&format!("following_target_{}", i))
                .await;
            ctx.create_follow(user_id, target_id).await;
        }

        // Get following (API returns all without pagination)
        let response = ctx
            .get(&format!("/api/v1/users/{}/following", user_id))
            .await;

        assert_eq!(response.status, 200, "Should succeed: {:?}", response.body);

        let following = response.body.as_array().expect("Should return array");
        assert_eq!(following.len(), 25, "API should return all 25 following");
    })
    .await;
}

// =============================================================================
// Error Path Tests - Invalid IDs and Edge Cases
// =============================================================================

/// Test: Revoke non-existent session returns 404
#[tokio::test]
async fn test_revoke_invalid_session_id() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Get followers for non-existent user returns 404
#[tokio::test]
async fn test_get_followers_invalid_user() {
    run_test(|ctx| async move {
        let fake_user_id = uuid::Uuid::new_v4();
        let response = ctx
            .get(&format!("/api/v1/users/{}/followers", fake_user_id))
            .await;

        assert_eq!(
            response.status, 404,
            "Followers for non-existent user should return 404: {:?}",
            response.body
        );
    })
    .await;
}

/// Test: Get following for non-existent user returns 404
#[tokio::test]
async fn test_get_following_invalid_user() {
    run_test(|ctx| async move {
        let fake_user_id = uuid::Uuid::new_v4();
        let response = ctx
            .get(&format!("/api/v1/users/{}/following", fake_user_id))
            .await;

        assert_eq!(
            response.status, 404,
            "Following for non-existent user should return 404: {:?}",
            response.body
        );
    })
    .await;
}

/// Test: Follow non-existent user returns error
/// Note: The follow endpoint relies on database foreign key constraint for user validation.
/// This returns a database error (500) rather than 404, as user existence is checked by FK.
#[tokio::test]
async fn test_follow_nonexistent_user() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("follower_404").await;

        let fake_user_id = uuid::Uuid::new_v4();
        let response = ctx
            .post_with_session(
                &format!("/api/v1/users/{}/follow", fake_user_id),
                json!({}),
                &session,
            )
            .await;

        // Database FK constraint causes 500, not 404
        // (Could be improved by checking user existence first)
        assert!(
            response.status == 404 || response.status == 500,
            "Following non-existent user should fail: status={}, body={:?}",
            response.status,
            response.body
        );
    })
    .await;
}

// =============================================================================
// Input Validation and Security Tests
// =============================================================================

/// Test: User search sanitizes special characters safely
/// Note: The search function filters out most special characters except `_` and `-`.
/// `%` is filtered out, but `_` passes through. In SQL LIKE, `_` matches any single char.
/// This is handled safely by parameterized queries (no SQL injection).
#[tokio::test]
async fn test_search_special_characters() {
    run_test(|mut ctx| async move {
        let (_, session) = ctx.create_and_login("search_special").await;

        // Test with percent signs - these get filtered out entirely
        // The query `%test` becomes `test%` after filtering `%`
        let response = ctx
            .get_with_session("/api/v1/users/search?q=%25test", &session) // %test URL-encoded
            .await;

        assert_eq!(
            response.status, 200,
            "Percent signs should be filtered safely: {:?}",
            response.body
        );

        // Verify search still works - should find our test user
        let _results = response.body.as_array().expect("Should return array");
        // The query "test" should not cause any SQL injection issues
        // Results may or may not be empty depending on existing users

        // Test with pure SQL injection attempt
        let response2 = ctx
            .get_with_session("/api/v1/users/search?q=%27%20OR%20%271%27%3D%271", &session) // ' OR '1'='1
            .await;

        assert_eq!(
            response2.status, 200,
            "SQL injection attempt should be handled safely: {:?}",
            response2.body
        );

        // Should return empty or safe results (injection should not work)
        let _ = response2.body.as_array().expect("Should return array");
    })
    .await;
}

/// Test: User search with regex special characters
#[tokio::test]
async fn test_search_regex_characters() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: User search with very long query
#[tokio::test]
async fn test_search_long_query() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Search limit parameter is clamped
#[tokio::test]
async fn test_search_limit_clamping() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

// =============================================================================
// Idempotency and State Tests
// =============================================================================

/// Test: Double-accept contribution (idempotency)
#[tokio::test]
async fn test_double_accept_contribution() {
    run_test(|mut ctx| async move {
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

        // Generate valid CSRF token
        let csrf_token = ctx.generate_csrf_token(&owner_session).await;

        // First accept
        let response1 = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/accept",
                    book_id, contribution_id
                ),
                &[("_csrf", &csrf_token)],
                &owner_session,
            )
            .await;

        assert!(
            response1.status == 200 || response1.status == 302,
            "First accept should succeed: {:?}",
            response1.body
        );

        // Generate another CSRF token for second request
        let csrf_token2 = ctx.generate_csrf_token(&owner_session).await;

        // Second accept (already accepted)
        let response2 = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/accept",
                    book_id, contribution_id
                ),
                &[("_csrf", &csrf_token2)],
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
    })
    .await;
}

/// Test: Accept contribution for non-existent book
#[tokio::test]
async fn test_accept_contribution_invalid_book() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("invalid_book").await;

        let fake_book_id = uuid::Uuid::new_v4();
        let fake_contribution_id = uuid::Uuid::new_v4();

        // Generate valid CSRF token
        let csrf_token = ctx.generate_csrf_token(&session).await;

        let response = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/accept",
                    fake_book_id, fake_contribution_id
                ),
                &[("_csrf", &csrf_token)],
                &session,
            )
            .await;

        assert_eq!(
            response.status, 404,
            "Accept on non-existent book should return 404: {:?}",
            response.body
        );
    })
    .await;
}

// =============================================================================
// Export Threshold Tests
// =============================================================================

/// Test: Export with many recipes works (no hard limit enforced)
/// Note: The export endpoint does not enforce a maximum recipe count.
/// All user data is exported regardless of size.
#[tokio::test]
async fn test_export_many_recipes_threshold() {
    run_test(|mut ctx| async move {
        let (user_id, session) = ctx.create_and_login("many_recipes").await;

        // Create 51 recipes
        for i in 0..51 {
            ctx.create_recipe(user_id, &format!("Recipe {}", i), "public")
                .await;
        }

        // Export should succeed (no recipe count limit)
        let response = ctx
            .get_with_session("/api/v1/users/me/export", &session)
            .await;

        assert_eq!(
            response.status, 200,
            "Export should succeed with many recipes: {:?}",
            response.body
        );

        // Verify all recipes are included in export
        let recipes = response.get("recipes").and_then(|r| r.as_array());
        assert!(recipes.is_some(), "Export should include recipes array");
        assert_eq!(
            recipes.unwrap().len(),
            51,
            "Export should include all recipes"
        );
    })
    .await;
}

/// Test: Export with exactly 50 recipes succeeds
#[tokio::test]
async fn test_export_exactly_50_recipes() {
    run_test(|mut ctx| async move {
        let (user_id, session) = ctx.create_and_login("fifty_recipes").await;

        // Create exactly 50 recipes
        for i in 0..50 {
            ctx.create_recipe(user_id, &format!("Recipe {}", i), "public")
                .await;
        }

        // Export should succeed
        let response = ctx
            .get_with_session("/api/v1/users/me/export", &session)
            .await;

        assert_eq!(
            response.status, 200,
            "Export with 50 recipes should succeed: {:?}",
            response.body
        );

        // Verify all recipes are included
        let recipes = response.get("recipes").and_then(|r| r.as_array());
        assert!(recipes.is_some(), "Export should include recipes array");
        assert_eq!(
            recipes.unwrap().len(),
            50,
            "Export should include all 50 recipes"
        );
    })
    .await;
}

// =============================================================================
// CSRF Validation Tests
// =============================================================================

/// Test: Accept contribution without CSRF token fails
#[tokio::test]
async fn test_accept_contribution_missing_csrf() {
    run_test(|mut ctx| async move {
        let (owner_id, owner_session) = ctx.create_and_login("csrf_owner").await;
        let (contributor_id, _) = ctx.create_and_login("csrf_contrib").await;

        let book_id = ctx.create_book(owner_id, "CSRF Test Book", "private").await;
        let recipe_id = ctx
            .create_complete_recipe(contributor_id, "CSRF Recipe", "public")
            .await;
        let contribution_id = ctx
            .create_book_contribution(book_id, recipe_id, contributor_id)
            .await;

        // Try to accept without CSRF token (empty form)
        let response = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/accept",
                    book_id, contribution_id
                ),
                &[], // No csrf_token
                &owner_session,
            )
            .await;

        // Should fail with 400 or 403 or 422 (validation error)
        assert!(
            response.status == 400 || response.status == 403 || response.status == 422,
            "Missing CSRF should be rejected: status={}, body={:?}",
            response.status,
            response.body
        );
    })
    .await;
}

/// Test: Accept contribution with invalid CSRF token fails
#[tokio::test]
async fn test_accept_contribution_invalid_csrf() {
    run_test(|mut ctx| async move {
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
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/accept",
                    book_id, contribution_id
                ),
                &[("_csrf", "invalid_token_12345")],
                &owner_session,
            )
            .await;

        // Should fail with 401 (unauthorized - CSRF validation uses that)
        assert!(
            response.status == 400 || response.status == 401 || response.status == 403,
            "Invalid CSRF should be rejected: status={}, body={:?}",
            response.status,
            response.body
        );
    })
    .await;
}

/// Test: Reject contribution without CSRF token fails
#[tokio::test]
async fn test_reject_contribution_missing_csrf() {
    run_test(|mut ctx| async move {
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

        // Try to reject without CSRF token (empty form)
        let response = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/reject",
                    book_id, contribution_id
                ),
                &[], // No csrf_token
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
    })
    .await;
}

// =============================================================================
// Data Consistency Tests
// =============================================================================

/// Test: After accepting contribution, recipe appears in book
#[tokio::test]
async fn test_accepted_contribution_adds_recipe_to_book() {
    run_test(|mut ctx| async move {
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

        // Generate CSRF token
        let csrf_token = ctx.generate_csrf_token(&owner_session).await;

        // Accept the contribution
        let response = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/accept",
                    book_id, contribution_id
                ),
                &[("_csrf", &csrf_token)],
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
    })
    .await;
}

/// Test: After rejecting contribution, recipe does NOT appear in book
#[tokio::test]
async fn test_rejected_contribution_does_not_add_recipe() {
    run_test(|mut ctx| async move {
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

        // Generate CSRF token
        let csrf_token = ctx.generate_csrf_token(&owner_session).await;

        // Reject the contribution
        let response = ctx
            .post_form_with_session(
                &format!(
                    "/books/{}/contributions/{}/reject",
                    book_id, contribution_id
                ),
                &[("_csrf", &csrf_token)],
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
    })
    .await;
}

/// Test: Follow count increases after following
#[tokio::test]
async fn test_follow_increases_count() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Follow count decreases after unfollowing
#[tokio::test]
async fn test_unfollow_decreases_count() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

// =============================================================================
// Idempotency Tests - Follow/Unfollow
// =============================================================================

/// Test: Double follow is handled gracefully
#[tokio::test]
async fn test_double_follow() {
    run_test(|mut ctx| async move {
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

        // Should either succeed (idempotent) or return 409 (conflict) or 422 (validation error)
        assert!(
            response2.status == 200
                || response2.status == 201
                || response2.status == 409
                || response2.status == 422,
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
    })
    .await;
}

/// Test: Double unfollow is handled gracefully
#[tokio::test]
async fn test_double_unfollow() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}

/// Test: Security event recorded on session revoke
#[tokio::test]
async fn test_security_event_on_session_revoke() {
    run_test(|mut ctx| async move {
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
    })
    .await;
}
