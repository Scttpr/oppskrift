//! Integration tests for recipe tags / categories
//!
//! Run with: cargo test --test tags_test

mod common;

use common::{fixtures, run_test};
use serde_json::json;

/// Creating a recipe with tags attaches them and returns them.
#[tokio::test]
async fn test_create_recipe_with_tags() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("tagger").await;

        let mut recipe = fixtures::create_test_recipe();
        recipe["tags"] = json!(["Dessert", "Quick & Easy"]);

        let response = ctx
            .post_with_session("/api/v1/recipes", recipe, &session)
            .await;

        assert_eq!(response.status, 201, "create failed: {:?}", response.body);

        let tags = response
            .get("tags")
            .and_then(|v| v.as_array())
            .expect("response should contain tags");
        let slugs: Vec<&str> = tags
            .iter()
            .filter_map(|t| t.get("slug").and_then(|s| s.as_str()))
            .collect();
        assert!(slugs.contains(&"dessert"), "tags: {:?}", slugs);
        assert!(slugs.contains(&"quick-easy"), "tags: {:?}", slugs);
    })
    .await;
}

/// Tagged public recipes are discoverable via the tag endpoint; the tag list
/// reports counts.
#[tokio::test]
async fn test_browse_recipes_by_tag() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("tag_browser").await;

        let mut recipe = fixtures::create_test_recipe();
        recipe["title"] = json!("Vegan Chili Special");
        recipe["visibility"] = json!("public");
        recipe["tags"] = json!(["Vegan"]);
        let created = ctx
            .post_with_session("/api/v1/recipes", recipe, &session)
            .await;
        assert_eq!(created.status, 201, "create failed: {:?}", created.body);

        // Recipes by tag slug
        let by_tag = ctx.get("/api/v1/tags/vegan/recipes").await;
        assert_eq!(by_tag.status, 200);
        let data = by_tag
            .get("data")
            .and_then(|v| v.as_array())
            .expect("data array");
        assert_eq!(data.len(), 1, "should find one vegan recipe: {:?}", data);

        // Tag list with counts
        let list = ctx.get("/api/v1/tags").await;
        assert_eq!(list.status, 200);
        let tags = list.body.as_array().expect("tags array");
        let vegan = tags
            .iter()
            .find(|t| t.get("slug").and_then(|s| s.as_str()) == Some("vegan"))
            .expect("vegan tag present");
        assert_eq!(vegan.get("recipe_count").and_then(|c| c.as_i64()), Some(1));
    })
    .await;
}

/// Updating a recipe replaces its tag set.
#[tokio::test]
async fn test_update_replaces_tags() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("tag_updater").await;

        let mut recipe = fixtures::create_test_recipe();
        recipe["tags"] = json!(["alpha", "beta"]);
        let created = ctx
            .post_with_session("/api/v1/recipes", recipe, &session)
            .await;
        let recipe_id = created
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string();

        // Replace tags
        let update = json!({ "title": "Updated", "tags": ["gamma"] });
        let response = ctx
            .server
            .put(&format!("/api/v1/recipes/{}", recipe_id))
            .add_cookie(cookie::Cookie::new("oppskrift_session", session.clone()))
            .json(&update)
            .await;
        assert_eq!(response.status_code().as_u16(), 200);

        let fetched = ctx.get(&format!("/api/v1/recipes/{}", recipe_id)).await;
        let slugs: Vec<&str> = fetched
            .get("tags")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .filter_map(|t| t.get("slug").and_then(|s| s.as_str()))
            .collect();
        assert_eq!(slugs, vec!["gamma"], "tags should be replaced: {:?}", slugs);
    })
    .await;
}

/// Private recipes do not surface in tag browsing.
#[tokio::test]
async fn test_tag_browse_excludes_private() {
    run_test(|mut ctx| async move {
        let (_user_id, session) = ctx.create_and_login("tag_priv").await;

        let mut recipe = fixtures::create_test_recipe();
        recipe["visibility"] = json!("private");
        recipe["tags"] = json!(["hushhush"]);
        let created = ctx
            .post_with_session("/api/v1/recipes", recipe, &session)
            .await;
        assert_eq!(created.status, 201, "create failed: {:?}", created.body);

        let by_tag = ctx.get("/api/v1/tags/hushhush/recipes").await;
        assert_eq!(by_tag.status, 200);
        let data = by_tag.get("data").and_then(|v| v.as_array()).unwrap();
        assert!(
            data.is_empty(),
            "private recipe must not appear: {:?}",
            data
        );
    })
    .await;
}
