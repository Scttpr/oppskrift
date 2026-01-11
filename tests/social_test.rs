//! Integration tests for Social API endpoints (follows, feed)
//!
//! Tests: Follow/unfollow users, activity feed
//! Run with: cargo test --test social_test

mod common;

use common::{run_test, TestContext};
use serde_json::json;

// =============================================================================
// Follow Tests (T036)
// =============================================================================

/// Test: Follow a user
#[tokio::test]
async fn test_follow_user() {
    run_test(|mut ctx| async move {
        // Create two users
        let email1 = TestContext::unique_email();
        let username1 = TestContext::unique_username();
        let user1_id = ctx
            .create_user(&email1, &username1, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let (_user2_id, session2) = ctx.create_and_login("follower").await;

        // User2 follows User1 - routes are merged at /api/v1, so path is /users/{id}/follow
        let response = ctx
            .post_with_session(
                &format!("/api/v1/users/{}/follow", user1_id),
                json!({}),
                &session2,
            )
            .await;

        assert!(
            response.status == 200 || response.status == 201,
            "Should follow user successfully: {:?}",
            response.body
        );
    })
    .await;
}

/// Test: Unfollow a user
#[tokio::test]
async fn test_unfollow_user() {
    run_test(|mut ctx| async move {
        // Create two users
        let email1 = TestContext::unique_email();
        let username1 = TestContext::unique_username();
        let user1_id = ctx
            .create_user(&email1, &username1, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let (user2_id, session2) = ctx.create_and_login("unfollower").await;

        // Create follow relationship directly
        ctx.create_follow(user2_id, user1_id).await;

        // Unfollow via API - routes are merged at /api/v1
        let response = ctx
            .server
            .delete(&format!("/api/v1/users/{}/follow", user1_id))
            .add_cookie(cookie::Cookie::new("oppskrift_session", session2.clone()))
            .await;

        assert_eq!(
            response.status_code().as_u16(),
            200,
            "Should unfollow user successfully"
        );
    })
    .await;
}

/// Test: Cannot follow yourself
#[tokio::test]
async fn test_cannot_follow_self() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        // Login
        let session = ctx
            .login_and_get_session(&email, "Xk9#mP2$vL5@nQ8!")
            .await
            .expect("Login should succeed");

        // Try to follow self - routes are merged at /api/v1
        let response = ctx
            .post_with_session(
                &format!("/api/v1/users/{}/follow", user_id),
                json!({}),
                &session,
            )
            .await;

        assert!(
            response.status == 400 || response.status == 422,
            "Should not allow following yourself, got status {}",
            response.status
        );
    })
    .await;
}

// =============================================================================
// Followers/Following Tests (T037)
// =============================================================================

/// Test: Get followers list
#[tokio::test]
async fn test_get_followers() {
    run_test(|mut ctx| async move {
        // Create target user
        let email1 = TestContext::unique_email();
        let username1 = TestContext::unique_username();
        let user1_id = ctx
            .create_user(&email1, &username1, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        // Create followers
        let email2 = TestContext::unique_email();
        let username2 = TestContext::unique_username();
        let user2_id = ctx
            .create_user(&email2, &username2, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        // User2 follows User1
        ctx.create_follow(user2_id, user1_id).await;

        // Get followers of User1 - uses user ID not username
        let response = ctx
            .get(&format!("/api/v1/users/{}/followers", user1_id))
            .await;

        assert_eq!(
            response.status, 200,
            "Should get followers list: {:?}",
            response.body
        );
    })
    .await;
}

/// Test: Get following list
#[tokio::test]
async fn test_get_following() {
    run_test(|mut ctx| async move {
        // Create users
        let email1 = TestContext::unique_email();
        let username1 = TestContext::unique_username();
        let user1_id = ctx
            .create_user(&email1, &username1, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        let email2 = TestContext::unique_email();
        let username2 = TestContext::unique_username();
        let user2_id = ctx
            .create_user(&email2, &username2, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        // User1 follows User2
        ctx.create_follow(user1_id, user2_id).await;

        // Get users that User1 is following - uses user ID not username
        let response = ctx
            .get(&format!("/api/v1/users/{}/following", user1_id))
            .await;

        assert_eq!(
            response.status, 200,
            "Should get following list: {:?}",
            response.body
        );
    })
    .await;
}

// =============================================================================
// Activity Feed Tests (T038)
// =============================================================================

/// Test: Get activity feed
#[tokio::test]
async fn test_get_activity_feed() {
    run_test(|mut ctx| async move {
        let (_, session) = ctx.create_and_login("feed_viewer").await;

        // Get feed (may be empty but should work)
        let response = ctx.get_with_session("/api/v1/feed", &session).await;

        assert_eq!(
            response.status, 200,
            "Should get activity feed: {:?}",
            response.body
        );
    })
    .await;
}

/// Test: Feed requires authentication
#[tokio::test]
async fn test_feed_requires_auth() {
    run_test(|ctx| async move {
        let response = ctx.get("/api/v1/feed").await;

        assert_eq!(response.status, 401, "Feed should require authentication");
    })
    .await;
}
