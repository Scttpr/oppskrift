//! Integration tests for recipe comments and ratings
//!
//! Run with: cargo test --test comments_ratings_test

mod common;

use common::{run_test, TestContext};
use serde_json::json;

/// Post a comment, then list it back.
#[tokio::test]
async fn test_add_and_list_comment() {
    run_test(|mut ctx| async move {
        let (user_id, session) = ctx.create_and_login("commenter").await;
        let recipe_id = ctx
            .create_recipe(user_id, "Commented Recipe", "public")
            .await;

        let post = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/comments", recipe_id),
                json!({ "body": "Loved this recipe!" }),
                &session,
            )
            .await;
        assert_eq!(post.status, 201, "post failed: {:?}", post.body);

        let list = ctx
            .get(&format!("/api/v1/recipes/{}/comments", recipe_id))
            .await;
        assert_eq!(list.status, 200);
        let arr = list.body.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(
            arr[0].get("body").and_then(|v| v.as_str()),
            Some("Loved this recipe!")
        );
    })
    .await;
}

/// Posting a comment requires authentication.
#[tokio::test]
async fn test_comment_requires_auth() {
    run_test(|mut ctx| async move {
        let email = TestContext::unique_email();
        let username = TestContext::unique_username();
        let user_id = ctx
            .create_user(&email, &username, "Xk9#mP2$vL5@nQ8!", true)
            .await;
        let recipe_id = ctx.create_recipe(user_id, "Public", "public").await;

        let post = ctx
            .post(
                &format!("/api/v1/recipes/{}/comments", recipe_id),
                json!({ "body": "hi" }),
            )
            .await;
        assert_eq!(post.status, 401, "should require auth");
    })
    .await;
}

/// Empty comment bodies are rejected.
#[tokio::test]
async fn test_empty_comment_rejected() {
    run_test(|mut ctx| async move {
        let (user_id, session) = ctx.create_and_login("emptycomment").await;
        let recipe_id = ctx.create_recipe(user_id, "R", "public").await;

        let post = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/comments", recipe_id),
                json!({ "body": "   " }),
                &session,
            )
            .await;
        assert!(
            post.status == 400 || post.status == 422,
            "expected validation error, got {}",
            post.status
        );
    })
    .await;
}

/// A non-author / non-owner cannot delete someone else's comment.
#[tokio::test]
async fn test_delete_comment_unauthorized() {
    run_test(|mut ctx| async move {
        let (owner_id, owner_session) = ctx.create_and_login("c_owner").await;
        let recipe_id = ctx.create_recipe(owner_id, "Owned", "public").await;

        let post = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/comments", recipe_id),
                json!({ "body": "owner comment" }),
                &owner_session,
            )
            .await;
        let comment_id = post.get("id").and_then(|v| v.as_str()).unwrap().to_string();

        let (_other_id, other_session) = ctx.create_and_login("c_other").await;
        let resp = ctx
            .server
            .delete(&format!(
                "/api/v1/recipes/{}/comments/{}",
                recipe_id, comment_id
            ))
            .add_cookie(cookie::Cookie::new("oppskrift_session", other_session))
            .await;
        assert_eq!(
            resp.status_code().as_u16(),
            404,
            "non-author delete must be denied"
        );

        // Author can delete their own.
        let ok = ctx
            .server
            .delete(&format!(
                "/api/v1/recipes/{}/comments/{}",
                recipe_id, comment_id
            ))
            .add_cookie(cookie::Cookie::new("oppskrift_session", owner_session))
            .await;
        assert_eq!(ok.status_code().as_u16(), 204, "author delete should work");
    })
    .await;
}

/// Setting a rating is idempotent per user and reflected in the summary.
#[tokio::test]
async fn test_set_and_update_rating() {
    run_test(|mut ctx| async move {
        let (user_id, session) = ctx.create_and_login("rater").await;
        let recipe_id = ctx.create_recipe(user_id, "Rated", "public").await;

        // First rating
        let r1 = ctx
            .server
            .put(&format!("/api/v1/recipes/{}/rating", recipe_id))
            .add_cookie(cookie::Cookie::new("oppskrift_session", session.clone()))
            .json(&json!({ "value": 4 }))
            .await;
        assert_eq!(r1.status_code().as_u16(), 200);

        // Update (upsert) — should not create a second rating
        let r2 = ctx
            .server
            .put(&format!("/api/v1/recipes/{}/rating", recipe_id))
            .add_cookie(cookie::Cookie::new("oppskrift_session", session.clone()))
            .json(&json!({ "value": 2 }))
            .await;
        assert_eq!(r2.status_code().as_u16(), 200);

        let summary = ctx
            .get(&format!("/api/v1/recipes/{}/rating", recipe_id))
            .await;
        assert_eq!(summary.get("count").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(summary.get("average").and_then(|v| v.as_f64()), Some(2.0));
    })
    .await;
}

/// Ratings outside 1–5 are rejected.
#[tokio::test]
async fn test_rating_out_of_range() {
    run_test(|mut ctx| async move {
        let (user_id, session) = ctx.create_and_login("badrater").await;
        let recipe_id = ctx.create_recipe(user_id, "R", "public").await;

        let resp = ctx
            .server
            .put(&format!("/api/v1/recipes/{}/rating", recipe_id))
            .add_cookie(cookie::Cookie::new("oppskrift_session", session))
            .json(&json!({ "value": 9 }))
            .await;
        assert!(
            resp.status_code().as_u16() == 400 || resp.status_code().as_u16() == 422,
            "out-of-range rating must be rejected, got {}",
            resp.status_code().as_u16()
        );
    })
    .await;
}
