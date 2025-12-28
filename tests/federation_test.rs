//! Integration tests for ActivityPub Federation endpoints
//!
//! Tests: WebFinger, Actor endpoints, ActivityPub collections
//! Run with: cargo test --test federation_test

mod common;

use axum::http::header::ACCEPT;
use common::{fixtures, TestContext};
use serde_json::Value;

// =============================================================================
// WebFinger Tests (T040)
// =============================================================================

/// Test: WebFinger lookup for existing user
#[tokio::test]
async fn test_webfinger_lookup_existing_user() {
    let mut ctx = TestContext::new().await;

    // Create a user
    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let _user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;

    // Query webfinger for the user
    let response = ctx
        .get(&format!(
            "/.well-known/webfinger?resource=acct:{}@localhost",
            username
        ))
        .await;

    assert_eq!(
        response.status, 200,
        "WebFinger should return 200 for existing user: {:?}",
        response.body
    );

    // Verify response structure
    assert!(
        response.get("subject").is_some(),
        "WebFinger response should have subject"
    );
    assert!(
        response.get("links").is_some(),
        "WebFinger response should have links"
    );

    ctx.cleanup().await;
}

/// Test: WebFinger lookup for non-existent user
#[tokio::test]
async fn test_webfinger_lookup_nonexistent_user() {
    let ctx = TestContext::new().await;

    let response = ctx
        .get("/.well-known/webfinger?resource=acct:nonexistent_user_xyz@localhost")
        .await;

    assert_eq!(
        response.status, 404,
        "WebFinger should return 404 for non-existent user"
    );
}

/// Test: WebFinger with invalid resource format
#[tokio::test]
async fn test_webfinger_invalid_resource_format() {
    let ctx = TestContext::new().await;

    // Missing acct: prefix
    let response = ctx
        .get("/.well-known/webfinger?resource=user@localhost")
        .await;

    assert_eq!(
        response.status, 400,
        "WebFinger should return 400 for invalid resource format"
    );
}

/// Test: WebFinger without @ separator
#[tokio::test]
async fn test_webfinger_missing_domain() {
    let ctx = TestContext::new().await;

    // Missing @ separator
    let response = ctx
        .get("/.well-known/webfinger?resource=acct:username")
        .await;

    assert_eq!(
        response.status, 400,
        "WebFinger should return 400 for missing domain"
    );
}

// =============================================================================
// Actor Endpoint Tests (T040)
// =============================================================================

/// Test: Get actor with ActivityPub Accept header
#[tokio::test]
async fn test_get_actor_with_activitypub_accept() {
    let mut ctx = TestContext::new().await;

    // Create a user
    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;

    // Get actor with ActivityPub Accept header
    let response = ctx
        .server
        .get(&format!("/ap/users/{}", user_id))
        .add_header(ACCEPT, "application/activity+json")
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        200,
        "Should return actor for ActivityPub request"
    );

    let body: Value = response.json();
    assert_eq!(body["type"], "Person", "Actor should be of type Person");
    assert!(
        body.get("inbox").is_some(),
        "Actor should have inbox endpoint"
    );
    assert!(
        body.get("outbox").is_some(),
        "Actor should have outbox endpoint"
    );
    assert!(
        body.get("publicKey").is_some(),
        "Actor should have public key"
    );

    ctx.cleanup().await;
}

/// Test: Get actor without ActivityPub Accept header returns 406
#[tokio::test]
async fn test_get_actor_without_activitypub_accept() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;

    // Get actor without ActivityPub Accept header
    let response = ctx
        .server
        .get(&format!("/ap/users/{}", user_id))
        .add_header(ACCEPT, "text/html")
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        406,
        "Should return 406 Not Acceptable without AP Accept header"
    );

    ctx.cleanup().await;
}

/// Test: Get non-existent actor
#[tokio::test]
async fn test_get_actor_not_found() {
    let ctx = TestContext::new().await;

    let fake_id = uuid::Uuid::new_v4();
    let response = ctx
        .server
        .get(&format!("/ap/users/{}", fake_id))
        .add_header(ACCEPT, "application/activity+json")
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        404,
        "Should return 404 for non-existent actor"
    );
}

// =============================================================================
// Outbox Tests (T040)
// =============================================================================

/// Test: Get user outbox collection
#[tokio::test]
async fn test_get_user_outbox() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;

    let response = ctx.get(&format!("/ap/users/{}/outbox", user_id)).await;

    assert_eq!(
        response.status, 200,
        "Should return outbox collection: {:?}",
        response.body
    );

    assert_eq!(
        response.get("type").and_then(|v| v.as_str()),
        Some("OrderedCollection"),
        "Outbox should be an OrderedCollection"
    );

    ctx.cleanup().await;
}

// =============================================================================
// Followers/Following Collection Tests (T040)
// =============================================================================

/// Test: Get user followers collection
#[tokio::test]
async fn test_get_user_followers_collection() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;

    let response = ctx.get(&format!("/ap/users/{}/followers", user_id)).await;

    assert_eq!(
        response.status, 200,
        "Should return followers collection: {:?}",
        response.body
    );

    assert_eq!(
        response.get("type").and_then(|v| v.as_str()),
        Some("OrderedCollection"),
        "Followers should be an OrderedCollection"
    );

    ctx.cleanup().await;
}

/// Test: Get user following collection
#[tokio::test]
async fn test_get_user_following_collection() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;

    let response = ctx.get(&format!("/ap/users/{}/following", user_id)).await;

    assert_eq!(
        response.status, 200,
        "Should return following collection: {:?}",
        response.body
    );

    assert_eq!(
        response.get("type").and_then(|v| v.as_str()),
        Some("OrderedCollection"),
        "Following should be an OrderedCollection"
    );

    ctx.cleanup().await;
}

// =============================================================================
// Recipe Object Tests (T040)
// =============================================================================

/// Test: Get recipe as ActivityPub object
#[tokio::test]
async fn test_get_recipe_as_activitypub_object() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;
    let recipe_id = ctx
        .create_complete_recipe(user_id, "AP Recipe", "public")
        .await;

    let response = ctx
        .server
        .get(&format!("/ap/recipes/{}", recipe_id))
        .add_header(ACCEPT, "application/activity+json")
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        200,
        "Should return recipe as AP object"
    );

    let body: Value = response.json();
    assert!(
        body.get("@context").is_some(),
        "Should have @context for JSON-LD"
    );
    assert!(
        body.get("attributedTo").is_some(),
        "Recipe should have attributedTo"
    );

    ctx.cleanup().await;
}

/// Test: Get recipe without ActivityPub Accept header
#[tokio::test]
async fn test_get_recipe_without_activitypub_accept() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;
    let recipe_id = ctx.create_recipe(user_id, "HTML Recipe", "public").await;

    let response = ctx
        .server
        .get(&format!("/ap/recipes/{}", recipe_id))
        .add_header(ACCEPT, "text/html")
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        406,
        "Should return 406 without AP Accept header"
    );

    ctx.cleanup().await;
}

// =============================================================================
// Book Object Tests (T040)
// =============================================================================

/// Test: Get book as ActivityPub collection
#[tokio::test]
async fn test_get_book_as_activitypub_collection() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;
    let book_id = ctx.create_book(user_id, "AP Book", "public").await;

    let response = ctx
        .server
        .get(&format!("/ap/books/{}", book_id))
        .add_header(ACCEPT, "application/activity+json")
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        200,
        "Should return book as AP collection"
    );

    let body: Value = response.json();
    assert!(
        body.get("@context").is_some(),
        "Should have @context for JSON-LD"
    );

    ctx.cleanup().await;
}

// =============================================================================
// Inbox Tests (T040)
// =============================================================================

/// Test: Inbox requires signature header
#[tokio::test]
async fn test_inbox_requires_signature() {
    let mut ctx = TestContext::new().await;

    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;

    // Create a follow activity from a mock remote actor
    let remote_actor = fixtures::mock_remote_actor("remote_alice", "remote.example");
    let local_actor_id = format!("http://localhost:3000/users/{}", user_id);
    let follow_activity =
        fixtures::mock_follow_activity(remote_actor["id"].as_str().unwrap(), &local_actor_id);

    // Send to inbox without signature
    let response = ctx
        .server
        .post(&format!("/ap/users/{}/inbox", user_id))
        .json(&follow_activity)
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        401,
        "Inbox should require signature header"
    );

    ctx.cleanup().await;
}

/// Test: Shared inbox requires signature
#[tokio::test]
async fn test_shared_inbox_requires_signature() {
    let ctx = TestContext::new().await;

    let follow_activity = fixtures::mock_follow_activity(
        "https://remote.example/users/alice",
        "http://localhost:3000/users/bob",
    );

    let response = ctx.server.post("/ap/inbox").json(&follow_activity).await;

    assert_eq!(
        response.status_code().as_u16(),
        401,
        "Shared inbox should require signature header"
    );
}

// =============================================================================
// Mock Actor/Activity Validation Tests
// =============================================================================

/// Test: Verify mock actor structure is valid
#[test]
fn test_mock_actor_structure() {
    let actor = fixtures::mock_remote_actor("alice", "example.com");

    assert_eq!(actor["type"], "Person");
    assert!(actor["id"]
        .as_str()
        .unwrap()
        .contains("example.com/users/alice"));
    assert!(actor["inbox"].as_str().is_some());
    assert!(actor["outbox"].as_str().is_some());
    assert!(actor["publicKey"].is_object());
}

/// Test: Verify mock follow activity structure
#[test]
fn test_mock_follow_activity_structure() {
    let activity = fixtures::mock_follow_activity(
        "https://remote.example/users/alice",
        "https://local.example/users/bob",
    );

    assert_eq!(activity["type"], "Follow");
    assert_eq!(activity["actor"], "https://remote.example/users/alice");
    assert_eq!(activity["object"], "https://local.example/users/bob");
    assert!(activity["id"].as_str().is_some());
}
