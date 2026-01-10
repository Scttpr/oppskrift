//! Handler Integration Tests (T057-T058)
//!
//! Tests for HTML page handlers, redirects, and auth flows.

mod common;

use common::TestContext;

// =============================================================================
// Legal Page Tests (T057)
// =============================================================================

#[tokio::test]
async fn test_about_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/about").await;

    assert_eq!(response.status_code(), 200);

    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap_or("");
    assert!(
        content_type.contains("text/html"),
        "Expected HTML content type, got: {}",
        content_type
    );
}

#[tokio::test]
async fn test_privacy_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/privacy").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_terms_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/terms").await;

    assert_eq!(response.status_code(), 200);
}

// =============================================================================
// Auth Page Tests (T057)
// =============================================================================

#[tokio::test]
async fn test_login_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/login").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_register_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/register").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_forgot_password_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/forgot-password").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_reset_password_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/reset-password").await;

    assert_eq!(response.status_code(), 200);
}

// =============================================================================
// Recipe Page Tests (T057)
// =============================================================================

#[tokio::test]
async fn test_recipe_list_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/recipes").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_new_recipe_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/recipes/new").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_recipe_not_found_returns_404() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/recipes/00000000-0000-0000-0000-000000000000")
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_recipe_view_with_valid_recipe() {
    let mut ctx = TestContext::new().await;

    // Create a user and recipe
    let user_id = ctx
        .create_user(
            &TestContext::unique_email(),
            &TestContext::unique_username(),
            "TestPass123!",
            true,
        )
        .await;
    let recipe_id = ctx.create_recipe(user_id, "Test Recipe", "public").await;

    let response = ctx.server.get(&format!("/recipes/{}", recipe_id)).await;

    assert_eq!(response.status_code(), 200);

    ctx.cleanup().await;
}

// =============================================================================
// Book Page Tests (T057)
// =============================================================================

#[tokio::test]
async fn test_book_list_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/books").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_new_book_page_returns_html() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/books/new").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_book_not_found_returns_404() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/books/00000000-0000-0000-0000-000000000000")
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_book_view_with_valid_book() {
    let mut ctx = TestContext::new().await;

    // Create a user and book
    let user_id = ctx
        .create_user(
            &TestContext::unique_email(),
            &TestContext::unique_username(),
            "TestPass123!",
            true,
        )
        .await;
    let book_id = ctx.create_book(user_id, "Test Book", "public").await;

    let response = ctx.server.get(&format!("/books/{}", book_id)).await;

    assert_eq!(response.status_code(), 200);

    ctx.cleanup().await;
}

// =============================================================================
// User Profile Page Tests (T057)
// =============================================================================

#[tokio::test]
async fn test_user_profile_not_found() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/users/00000000-0000-0000-0000-000000000000")
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_user_profile_with_valid_user() {
    let mut ctx = TestContext::new().await;

    // Create a test user
    let user_id = ctx
        .create_user(
            &TestContext::unique_email(),
            &TestContext::unique_username(),
            "TestPass123!",
            true,
        )
        .await;

    let response = ctx.server.get(&format!("/users/{}", user_id)).await;

    assert_eq!(response.status_code(), 200);

    ctx.cleanup().await;
}

// =============================================================================
// Protected Route Tests (T058)
// =============================================================================

#[tokio::test]
async fn test_feed_requires_authentication() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/feed").await;

    // Feed page requires auth - should redirect or return 401
    let status = response.status_code().as_u16();
    assert!(
        status == 401 || status == 302 || status == 303,
        "Expected redirect or 401 for unauthenticated feed access, got {}",
        status
    );
}

#[tokio::test]
async fn test_saved_recipes_requires_authentication() {
    let ctx = TestContext::new().await;

    let response = ctx
        .server
        .get("/users/00000000-0000-0000-0000-000000000000/saved")
        .await;

    // Saved recipes require auth
    let status = response.status_code().as_u16();
    assert!(
        status == 401 || status == 302 || status == 303,
        "Expected redirect or 401 for unauthenticated saved recipes access, got {}",
        status
    );
}

// =============================================================================
// Pagination Tests (T058)
// =============================================================================

#[tokio::test]
async fn test_recipe_list_pagination() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/recipes?page=1&page_size=10").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_book_list_pagination() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/books?page=1&page_size=20").await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_recipe_list_second_page() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/recipes?page=2&page_size=10").await;

    assert_eq!(response.status_code(), 200);
}

// =============================================================================
// Content Type Tests (T058)
// =============================================================================

#[tokio::test]
async fn test_recipes_page_returns_html_content_type() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/recipes").await;

    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or(""));

    assert!(
        content_type.is_some_and(|ct| ct.contains("text/html")),
        "Page /recipes should return HTML content type"
    );
}

#[tokio::test]
async fn test_books_page_returns_html_content_type() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/books").await;

    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or(""));

    assert!(
        content_type.is_some_and(|ct| ct.contains("text/html")),
        "Page /books should return HTML content type"
    );
}

#[tokio::test]
async fn test_login_page_returns_html_content_type() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/login").await;

    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or(""));

    assert!(
        content_type.is_some_and(|ct| ct.contains("text/html")),
        "Page /login should return HTML content type"
    );
}

// =============================================================================
// Home/Index Page Tests (T058)
// =============================================================================

#[tokio::test]
async fn test_home_page_accessible() {
    let ctx = TestContext::new().await;

    let response = ctx.server.get("/").await;

    // Home page should return OK or redirect
    let status = response.status_code().as_u16();
    assert!(
        status == 200 || status == 302 || status == 303,
        "Home page should be accessible, got {}",
        status
    );
}

// =============================================================================
// Authenticated Handler Tests (T058)
// =============================================================================

#[tokio::test]
async fn test_feed_accessible_when_authenticated() {
    let mut ctx = TestContext::new().await;

    // Create and login user
    let (_, session) = ctx.create_and_login("feed_user").await;

    let response = ctx
        .server
        .get("/feed")
        .add_cookie(cookie::Cookie::new("oppskrift_session", session))
        .await;

    assert_eq!(response.status_code(), 200);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_saved_recipes_accessible_to_owner() {
    let mut ctx = TestContext::new().await;

    // Create and login user
    let (user_id, session) = ctx.create_and_login("saved_user").await;

    let response = ctx
        .server
        .get(&format!("/users/{}/saved", user_id))
        .add_cookie(cookie::Cookie::new("oppskrift_session", session))
        .await;

    assert_eq!(response.status_code(), 200);

    ctx.cleanup().await;
}

#[tokio::test]
async fn test_saved_recipes_forbidden_to_other_user() {
    let mut ctx = TestContext::new().await;

    // Create two users
    let user1_id = ctx
        .create_user(
            &TestContext::unique_email(),
            &TestContext::unique_username(),
            "TestPass123!",
            true,
        )
        .await;

    let (_, user2_session) = ctx.create_and_login("other_user").await;

    // User 2 tries to access User 1's saved recipes
    let response = ctx
        .server
        .get(&format!("/users/{}/saved", user1_id))
        .add_cookie(cookie::Cookie::new("oppskrift_session", user2_session))
        .await;

    // Should be forbidden
    assert_eq!(response.status_code(), 403);

    ctx.cleanup().await;
}
