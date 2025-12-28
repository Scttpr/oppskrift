//! Integration tests for Recipe API endpoints
//!
//! Tests: GET/POST/PUT/DELETE /api/v1/recipes
//! Run with: cargo test --test recipes_test

mod common;

use common::{fixtures, TestContext};
use serde_json::json;

// =============================================================================
// Recipe Creation Tests (T027-T028)
// =============================================================================

/// Test: Create recipe with valid data
#[tokio::test]
async fn test_create_recipe_success() {
    let mut ctx = TestContext::new().await;

    // Create and login a user
    let (user_id, session) = ctx.create_and_login("recipe_creator").await;

    // Create recipe via API
    let recipe_data = fixtures::create_test_recipe();
    let response = ctx
        .post_with_session("/api/v1/recipes", recipe_data, &session)
        .await;

    assert_eq!(
        response.status, 201,
        "Expected 201 Created: {:?}",
        response.body
    );
    assert!(
        response.get("id").is_some(),
        "Response should contain recipe id"
    );
    assert!(
        response.get("title").is_some(),
        "Response should contain title"
    );

    ctx.cleanup().await;
}

/// Test: Create recipe without authentication fails
#[tokio::test]
async fn test_create_recipe_requires_auth() {
    let ctx = TestContext::new().await;

    let recipe_data = fixtures::create_test_recipe();
    let response = ctx.post("/api/v1/recipes", recipe_data).await;

    assert_eq!(response.status, 401, "Should require authentication");
}

/// Test: Create recipe with missing required field
#[tokio::test]
async fn test_create_recipe_missing_title() {
    let mut ctx = TestContext::new().await;

    let (_user_id, session) = ctx.create_and_login("recipe_validator").await;

    // Missing title field should fail with 400 or 422
    let invalid_recipe = json!({
        "visibility": "public",
        "ingredients": [],
        "instructions": []
    });

    let response = ctx
        .post_with_session("/api/v1/recipes", invalid_recipe, &session)
        .await;

    assert!(
        response.status == 400 || response.status == 422,
        "Expected 400/422 for missing title, got {}: {:?}",
        response.status,
        response.body
    );

    ctx.cleanup().await;
}

// =============================================================================
// Recipe Read Tests (T029)
// =============================================================================

/// Test: Get public recipe
#[tokio::test]
async fn test_get_public_recipe() {
    let mut ctx = TestContext::new().await;

    // Create user and recipe
    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;
    let recipe_id = ctx
        .create_complete_recipe(user_id, "Public Recipe", "public")
        .await;

    // Get recipe without auth (should work for public)
    let response = ctx.get(&format!("/api/v1/recipes/{}", recipe_id)).await;

    assert_eq!(
        response.status, 200,
        "Should get public recipe: {:?}",
        response.body
    );
    assert_eq!(
        response.get("title").and_then(|v| v.as_str()),
        Some("Public Recipe"),
        "Should return correct title"
    );

    ctx.cleanup().await;
}

/// Test: Get private recipe without auth
#[tokio::test]
async fn test_get_private_recipe_without_auth() {
    let mut ctx = TestContext::new().await;

    // Create user and private recipe
    let email = TestContext::unique_email();
    let username = TestContext::unique_username();
    let user_id = ctx
        .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
        .await;
    let recipe_id = ctx
        .create_recipe(user_id, "Private Recipe", "private")
        .await;

    // Try to get private recipe without auth
    let response = ctx.get(&format!("/api/v1/recipes/{}", recipe_id)).await;

    assert!(
        response.status == 403 || response.status == 404,
        "Should not access private recipe without auth, got {}",
        response.status
    );

    ctx.cleanup().await;
}

/// Test: Get non-existent recipe
#[tokio::test]
async fn test_get_recipe_not_found() {
    let ctx = TestContext::new().await;

    let fake_id = uuid::Uuid::new_v4();
    let response = ctx.get(&format!("/api/v1/recipes/{}", fake_id)).await;

    assert_eq!(
        response.status, 404,
        "Should return 404 for non-existent recipe"
    );
}

// =============================================================================
// Recipe Update Tests (T030)
// =============================================================================

/// Test: Update own recipe
#[tokio::test]
async fn test_update_recipe_success() {
    let mut ctx = TestContext::new().await;

    let (user_id, session) = ctx.create_and_login("recipe_updater").await;
    let recipe_id = ctx.create_recipe(user_id, "Original Title", "public").await;

    // Update the recipe
    let update_data = json!({
        "title": "Updated Title",
        "description": "Updated description"
    });

    let response = ctx
        .server
        .put(&format!("/api/v1/recipes/{}", recipe_id))
        .add_cookie(cookie::Cookie::new("oppskrift_session", session.clone()))
        .json(&update_data)
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        200,
        "Should update recipe successfully"
    );

    ctx.cleanup().await;
}

// =============================================================================
// Recipe Delete Tests (T031)
// =============================================================================

/// Test: Delete own recipe
#[tokio::test]
async fn test_delete_recipe_success() {
    let mut ctx = TestContext::new().await;

    let (user_id, session) = ctx.create_and_login("recipe_deleter").await;
    let recipe_id = ctx.create_recipe(user_id, "To Delete", "public").await;

    // Delete the recipe
    let response = ctx
        .server
        .delete(&format!("/api/v1/recipes/{}", recipe_id))
        .add_cookie(cookie::Cookie::new("oppskrift_session", session.clone()))
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        204,
        "Should delete recipe successfully"
    );

    // Verify it's gone
    let get_response = ctx.get(&format!("/api/v1/recipes/{}", recipe_id)).await;
    assert_eq!(get_response.status, 404, "Recipe should no longer exist");

    ctx.cleanup().await;
}

/// Test: Cannot delete another user's recipe
#[tokio::test]
async fn test_delete_recipe_unauthorized() {
    let mut ctx = TestContext::new().await;

    // Create recipe owner
    let email1 = TestContext::unique_email();
    let username1 = TestContext::unique_username();
    let owner_id = ctx
        .create_user(&email1, &username1, "Xk9#mP2$vL5@nQ8!", true)
        .await;
    let recipe_id = ctx
        .create_recipe(owner_id, "Owner's Recipe", "public")
        .await;

    // Create different user and login
    let (_other_id, other_session) = ctx.create_and_login("other_user").await;

    // Try to delete recipe as other user
    let response = ctx
        .server
        .delete(&format!("/api/v1/recipes/{}", recipe_id))
        .add_cookie(cookie::Cookie::new(
            "oppskrift_session",
            other_session.clone(),
        ))
        .await;

    assert_eq!(
        response.status_code().as_u16(),
        403,
        "Should not delete another user's recipe"
    );

    ctx.cleanup().await;
}
