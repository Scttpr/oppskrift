//! Integration tests for recipe full-text search
//!
//! Tests: GET /api/v1/recipes/search
//! Run with: cargo test --test recipe_search_test

mod common;

use common::{run_test, TestContext};

/// Search returns matching public recipes and excludes non-matches.
#[tokio::test]
async fn test_search_matches_title() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        ctx.create_recipe(user_id, "Zphlogiston Chocolate Cake", "public")
            .await;
        ctx.create_recipe(user_id, "Plain Garlic Bread", "public")
            .await;

        let response = ctx.get("/api/v1/recipes/search?q=zphlogiston").await;

        assert_eq!(
            response.status, 200,
            "search should succeed: {:?}",
            response.body
        );

        let data = response
            .get("data")
            .and_then(|v| v.as_array())
            .expect("response should have data array");
        assert_eq!(data.len(), 1, "should match exactly one recipe: {:?}", data);
        assert_eq!(
            data[0].get("title").and_then(|v| v.as_str()),
            Some("Zphlogiston Chocolate Cake")
        );
    })
    .await;
}

/// Search does not return private recipes.
#[tokio::test]
async fn test_search_excludes_private() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;

        ctx.create_recipe(user_id, "Qwizzle Secret Stew", "private")
            .await;

        let response = ctx.get("/api/v1/recipes/search?q=qwizzle").await;

        assert_eq!(response.status, 200);
        let data = response
            .get("data")
            .and_then(|v| v.as_array())
            .expect("response should have data array");
        assert!(
            data.is_empty(),
            "private recipe must not appear: {:?}",
            data
        );
    })
    .await;
}

/// A query with no matches returns an empty result set.
#[tokio::test]
async fn test_search_no_matches() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;
        ctx.create_recipe(user_id, "Ordinary Pancakes", "public")
            .await;

        let response = ctx
            .get("/api/v1/recipes/search?q=nonexistentterm12345")
            .await;

        assert_eq!(response.status, 200);
        let total = response
            .get("pagination")
            .and_then(|p| p.get("total_items"))
            .and_then(|v| v.as_u64());
        assert_eq!(total, Some(0), "should report zero results");
    })
    .await;
}

/// An empty query returns an empty result set without error.
#[tokio::test]
async fn test_search_empty_query() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;
        ctx.create_recipe(user_id, "Some Public Recipe", "public")
            .await;

        let response = ctx.get("/api/v1/recipes/search?q=").await;

        assert_eq!(response.status, 200);
        let data = response
            .get("data")
            .and_then(|v| v.as_array())
            .expect("response should have data array");
        assert!(data.is_empty(), "empty query returns no results");
    })
    .await;
}
