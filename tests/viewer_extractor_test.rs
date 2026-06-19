//! Integration tests for the `Viewer` extractor.
//!
//! Exercises a `Viewer`-backed HTML route two ways: authenticated (the
//! extractor loads and renders the signed-in user) and unauthenticated
//! (rejected by the underlying `AuthUser` with 401).
//!
//! The "valid session but deleted user" -> 404 branch is defensive only:
//! `sessions.user_id` is `ON DELETE CASCADE`, so deleting the user removes the
//! session (yielding 401, not 404), and the same NOT NULL foreign key forbids
//! inserting an orphan session. That state is unconstructible through the
//! database, so there is no integration test for it.

mod common;

use common::{run_test, TestContext};

#[tokio::test]
async fn test_viewer_renders_signed_in_user() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let password = "Xk9#mP2$vL5@nQ8!";
        ctx.create_user(&email, &username, password, true).await;
        let session = ctx
            .login_and_get_session(&email, password)
            .await
            .expect("login should succeed");

        let response = ctx
            .server
            .get("/settings/profile")
            .add_cookie(cookie::Cookie::new("oppskrift_session", session))
            .await;

        assert_eq!(response.status_code(), 200);
        assert!(
            response.text().contains(&username),
            "profile page should render the viewer's username"
        );
    })
    .await;
}

#[tokio::test]
async fn test_viewer_rejects_unauthenticated() {
    run_test(|ctx| async move {
        let response = ctx.server.get("/settings/profile").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}
