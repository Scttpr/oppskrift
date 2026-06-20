//! Integration tests for the `Authorized<R, L>` extractor.
//!
//! Exercises the HTML recipe/book page routes that now load the resource and
//! evaluate its permission at one seam:
//! - `View`: owner sees a private resource (200), anonymous is told it does not
//!   exist (404), and a public resource is visible to anyone (200).
//! - `Edit`: anonymous is rejected up front (401, login), a non-owner is told
//!   the resource does not exist (404).

mod common;

use common::run_test;

#[tokio::test]
async fn test_owner_views_own_private_recipe_page() {
    run_test(|mut ctx| async move {
        let (owner_id, session) = ctx.create_and_login("owner").await;
        let recipe_id = ctx.create_recipe(owner_id, "Secret Sauce", "private").await;

        let response = ctx
            .server
            .get(&format!("/recipes/{}", recipe_id))
            .add_cookie(cookie::Cookie::new("oppskrift_session", session))
            .await;

        assert_eq!(response.status_code(), 200);
        assert!(
            response.text().contains("Secret Sauce"),
            "owner should see their private recipe"
        );
    })
    .await;
}

#[tokio::test]
async fn test_private_recipe_page_hidden_from_anonymous() {
    run_test(|mut ctx| async move {
        let (owner_id, _session) = ctx.create_and_login("owner").await;
        let recipe_id = ctx.create_recipe(owner_id, "Secret Sauce", "private").await;

        let response = ctx.server.get(&format!("/recipes/{}", recipe_id)).await;

        assert_eq!(
            response.status_code(),
            404,
            "anonymous viewer must not learn a private recipe exists"
        );
    })
    .await;
}

#[tokio::test]
async fn test_public_recipe_page_visible_to_anonymous() {
    run_test(|mut ctx| async move {
        let (owner_id, _session) = ctx.create_and_login("owner").await;
        let recipe_id = ctx.create_recipe(owner_id, "Shared Stew", "public").await;

        let response = ctx.server.get(&format!("/recipes/{}", recipe_id)).await;

        assert_eq!(response.status_code(), 200);
        assert!(response.text().contains("Shared Stew"));
    })
    .await;
}

#[tokio::test]
async fn test_recipe_edit_page_requires_auth() {
    run_test(|mut ctx| async move {
        let (owner_id, _session) = ctx.create_and_login("owner").await;
        let recipe_id = ctx.create_recipe(owner_id, "Secret Sauce", "public").await;

        let response = ctx
            .server
            .get(&format!("/recipes/{}/edit", recipe_id))
            .await;

        assert_eq!(
            response.status_code(),
            401,
            "anonymous visitor must be rejected before the edit page loads"
        );
    })
    .await;
}

#[tokio::test]
async fn test_recipe_edit_page_forbidden_for_non_owner() {
    run_test(|mut ctx| async move {
        let (owner_id, _owner_session) = ctx.create_and_login("owner").await;
        let recipe_id = ctx.create_recipe(owner_id, "Secret Sauce", "public").await;
        let (_other_id, other_session) = ctx.create_and_login("other").await;

        let response = ctx
            .server
            .get(&format!("/recipes/{}/edit", recipe_id))
            .add_cookie(cookie::Cookie::new("oppskrift_session", other_session))
            .await;

        assert_eq!(
            response.status_code(),
            404,
            "a non-owner without edit rights must get 404, not the edit form"
        );
    })
    .await;
}
