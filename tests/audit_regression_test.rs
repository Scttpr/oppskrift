//! Regression tests for the security/authorization fixes from the code-quality audit (PR #54).
//!
//! These lock in:
//!   - cross-recipe image IDOR (image ops scoped to their recipe)
//!   - private-resource disclosure via the HTML view pages
//!   - the recipe edit page now requiring authentication
//!
//! Run with: cargo test --test audit_regression_test

mod common;

use common::{run_test, TestContext};
use uuid::Uuid;

const PW: &str = "Xk9#mP2$vL5@nQ8!";

/// IDOR: setting an image primary must be scoped to the recipe in the path.
/// A user must not be able to touch an image belonging to a different recipe.
#[tokio::test]
async fn test_set_primary_image_is_scoped_to_recipe() {
    run_test(|mut ctx| async move {
        let owner_a = ctx
            .create_user(
                &TestContext::unique_email(),
                &TestContext::unique_username(),
                PW,
                true,
            )
            .await;
        let owner_b = ctx
            .create_user(
                &TestContext::unique_email(),
                &TestContext::unique_username(),
                PW,
                true,
            )
            .await;
        let recipe_a = ctx.create_recipe(owner_a, "Recipe A", "public").await;
        let recipe_b = ctx.create_recipe(owner_b, "Recipe B", "public").await;

        // An image that belongs to recipe B.
        let image_b: Uuid = sqlx::query_scalar(
            "INSERT INTO recipe_images (recipe_id, url, position, is_primary) \
             VALUES ($1, $2, 1, true) RETURNING id",
        )
        .bind(recipe_b)
        .bind("http://example.com/b.webp")
        .fetch_one(&ctx.db)
        .await
        .expect("insert image");

        // Cross-recipe call (image_b does not belong to recipe_a) must fail.
        let cross = oppskrift::services::ImageService::set_primary(&ctx.db, recipe_a, image_b).await;
        assert!(
            cross.is_err(),
            "set_primary across recipes must not succeed (IDOR)"
        );

        // Image B is untouched and still belongs to recipe B.
        let (still_primary, owner_recipe): (bool, Uuid) =
            sqlx::query_as("SELECT is_primary, recipe_id FROM recipe_images WHERE id = $1")
                .bind(image_b)
                .fetch_one(&ctx.db)
                .await
                .expect("image still exists");
        assert!(still_primary, "image B must be unchanged");
        assert_eq!(owner_recipe, recipe_b, "image B must still belong to recipe B");

        // The legitimate owner's call (correct recipe) succeeds.
        let ok = oppskrift::services::ImageService::set_primary(&ctx.db, recipe_b, image_b).await;
        assert!(ok.is_ok(), "owner setting primary on their own image should work");
    })
    .await;
}

/// Disclosure: the HTML recipe view page must not render a private recipe to an anonymous viewer.
#[tokio::test]
async fn test_private_recipe_view_page_hidden_from_anonymous() {
    run_test(|mut ctx| async move {
        let owner = ctx
            .create_user(
                &TestContext::unique_email(),
                &TestContext::unique_username(),
                PW,
                true,
            )
            .await;
        let recipe = ctx.create_recipe(owner, "Secret Recipe", "private").await;

        let res = ctx.get(&format!("/recipes/{}", recipe)).await;
        assert!(
            res.status == 403 || res.status == 404,
            "anonymous must not view a private recipe page, got {}",
            res.status
        );
    })
    .await;
}

/// Disclosure: the HTML book view page must not render a private book to an anonymous viewer.
#[tokio::test]
async fn test_private_book_view_page_hidden_from_anonymous() {
    run_test(|mut ctx| async move {
        let owner = ctx
            .create_user(
                &TestContext::unique_email(),
                &TestContext::unique_username(),
                PW,
                true,
            )
            .await;
        let book = ctx.create_book(owner, "Secret Book", "private").await;

        let res = ctx.get(&format!("/books/{}", book)).await;
        assert!(
            res.status == 403 || res.status == 404,
            "anonymous must not view a private book page, got {}",
            res.status
        );
    })
    .await;
}

/// Authz: the recipe edit form requires authentication (previously loadable by anyone).
#[tokio::test]
async fn test_recipe_edit_page_requires_auth() {
    run_test(|mut ctx| async move {
        let owner = ctx
            .create_user(
                &TestContext::unique_email(),
                &TestContext::unique_username(),
                PW,
                true,
            )
            .await;
        let recipe = ctx.create_recipe(owner, "Editable", "public").await;

        // No session -> the edit form must not be served (401/403/404 or a redirect to login).
        let res = ctx.get(&format!("/recipes/{}/edit", recipe)).await;
        assert!(
            res.status != 200,
            "anonymous must not load the recipe edit form, got {}",
            res.status
        );
    })
    .await;
}
