//! Integration tests for the recipe serving scaler (client-side scaling UI).
//!
//! These assert that the recipe view page renders the scaler control and the
//! data attributes the scaling JavaScript relies on. The arithmetic itself runs
//! in the browser.
//!
//! Run with: cargo test --test recipe_scaling_test

mod common;

use common::{run_test, TestContext};

#[tokio::test]
async fn test_recipe_view_renders_scaler_and_data_attrs() {
    run_test(|mut ctx| async move {
        let user_id = ctx
            .create_user(
                &TestContext::unique_email(),
                &TestContext::unique_username(),
                "TestPass123!",
                true,
            )
            .await;
        let recipe_id = ctx
            .create_recipe(user_id, "Scalable Recipe", "public")
            .await;
        ctx.create_ingredients(recipe_id, 3).await;

        let response = ctx.server.get(&format!("/recipes/{}", recipe_id)).await;
        assert_eq!(response.status_code(), 200);

        let html = response.text();
        assert!(
            html.contains("id=\"recipe-scaler\""),
            "view page should render the serving scaler"
        );
        assert!(
            html.contains("data-base-qty"),
            "ingredient quantities should carry data-base-qty for scaling"
        );
        assert!(
            html.contains("data-base-servings"),
            "servings should carry data-base-servings for scaling"
        );
        // The base quantity must remain present unscaled in the initial render.
        assert!(
            html.contains("scale-btn"),
            "scale multiplier buttons should be present"
        );
    })
    .await;
}

#[tokio::test]
async fn test_scaler_present_even_without_quantities() {
    run_test(|mut ctx| async move {
        let user_id = ctx
            .create_user(
                &TestContext::unique_email(),
                &TestContext::unique_username(),
                "TestPass123!",
                true,
            )
            .await;
        let recipe_id = ctx.create_recipe(user_id, "No Qty Recipe", "public").await;

        let response = ctx.server.get(&format!("/recipes/{}", recipe_id)).await;
        assert_eq!(response.status_code(), 200);
        let html = response.text();
        assert!(html.contains("id=\"recipe-scaler\""));
    })
    .await;
}
