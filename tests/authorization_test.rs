//! Authorization Integration Tests (T101)
//!
//! Tests for the ABAC (Attribute-Based Access Control) authorization system.

mod common;

use common::run_test;

// =============================================================================
// Owner Access Tests
// =============================================================================

#[tokio::test]
async fn test_owner_can_access_own_recipe() {
    run_test(|mut ctx| async move {
        // Create owner and login
        let (owner_id, session) = ctx.create_and_login("owner").await;

        // Create a private recipe
        let recipe_id = ctx.create_recipe(owner_id, "Owner Recipe", "private").await;

        // Owner should be able to access their own recipe
        let response = ctx
            .get_with_session(&format!("/api/v1/recipes/{}", recipe_id), &session)
            .await;

        assert_eq!(response.status, 200, "Owner should access own recipe");
    })
    .await;
}

#[tokio::test]
async fn test_owner_can_delete_own_recipe() {
    run_test(|mut ctx| async move {
        // Create owner and login
        let (owner_id, session) = ctx.create_and_login("owner").await;

        // Create a recipe
        let recipe_id = ctx.create_recipe(owner_id, "Delete Test", "private").await;

        // Owner should be able to delete
        let response = ctx
            .delete_with_session(&format!("/api/v1/recipes/{}", recipe_id), &session)
            .await;

        assert!(
            response.status == 200 || response.status == 204,
            "Owner should delete own recipe"
        );
    })
    .await;
}

// =============================================================================
// Public Visibility Tests
// =============================================================================

#[tokio::test]
async fn test_public_recipe_accessible_to_anyone() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, _) = ctx.create_and_login("owner").await;

        // Create a public recipe
        let recipe_id = ctx.create_recipe(owner_id, "Public Recipe", "public").await;

        // Anonymous user should be able to view
        let response = ctx.get(&format!("/api/v1/recipes/{}", recipe_id)).await;

        assert_eq!(
            response.status, 200,
            "Public recipe should be viewable by anyone"
        );
    })
    .await;
}

#[tokio::test]
async fn test_private_recipe_not_accessible_to_others() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, _) = ctx.create_and_login("owner").await;

        // Create another user
        let (_, other_session) = ctx.create_and_login("other").await;

        // Create a private recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "Private Recipe", "private")
            .await;

        // Other user should NOT be able to view (returns 404 per ABAC spec)
        let response = ctx
            .get_with_session(&format!("/api/v1/recipes/{}", recipe_id), &other_session)
            .await;

        assert_eq!(
            response.status, 404,
            "Private recipe should return 404 for unauthorized users"
        );
    })
    .await;
}

// =============================================================================
// Direct User Permission Tests
// =============================================================================

#[tokio::test]
async fn test_shared_recipe_accessible_to_granted_user() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;

        // Create grantee
        let (grantee_id, grantee_session) = ctx.create_and_login("grantee").await;

        // Create a private recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "Shared Recipe", "private")
            .await;

        // Grant view permission to grantee via API
        let grant_response = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": grantee_id,
                    "permission_level": "view"
                }),
                &owner_session,
            )
            .await;

        assert!(
            grant_response.is_success(),
            "Owner should be able to grant permission"
        );

        // Grantee should now be able to view
        let response = ctx
            .get_with_session(&format!("/api/v1/recipes/{}", recipe_id), &grantee_session)
            .await;

        assert_eq!(
            response.status, 200,
            "User with view permission should access recipe"
        );
    })
    .await;
}

#[tokio::test]
async fn test_edit_permission_allows_update() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;

        // Create editor
        let (editor_id, editor_session) = ctx.create_and_login("editor").await;

        // Create a private recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "Editable Recipe", "private")
            .await;

        // Grant edit permission
        let _ = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": editor_id,
                    "permission_level": "edit"
                }),
                &owner_session,
            )
            .await;

        // Editor should be able to update (API uses PUT, requires full payload)
        let response = ctx
            .put_with_session(
                &format!("/api/v1/recipes/{}", recipe_id),
                serde_json::json!({
                    "title": "Updated by Editor",
                    "description": "Updated description",
                    "prep_time_min": 15,
                    "cook_time_min": 30,
                    "servings": "4",
                    "difficulty": "medium",
                    "visibility": "private"
                }),
                &editor_session,
            )
            .await;

        assert!(
            response.is_success(),
            "User with edit permission should update recipe, got status: {}",
            response.status
        );
    })
    .await;
}

#[tokio::test]
async fn test_view_permission_does_not_allow_update() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;

        // Create viewer
        let (viewer_id, viewer_session) = ctx.create_and_login("viewer").await;

        // Create a private recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "View Only Recipe", "private")
            .await;

        // Grant only view permission
        let _ = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": viewer_id,
                    "permission_level": "view"
                }),
                &owner_session,
            )
            .await;

        // Viewer should NOT be able to update (404 per ABAC spec)
        let response = ctx
            .put_with_session(
                &format!("/api/v1/recipes/{}", recipe_id),
                serde_json::json!({
                    "title": "Should Fail",
                    "description": "Should not work",
                    "prep_time_min": 15,
                    "cook_time_min": 30,
                    "servings": "4",
                    "difficulty": "medium",
                    "visibility": "private"
                }),
                &viewer_session,
            )
            .await;

        assert_eq!(
            response.status, 404,
            "User with only view permission should not update"
        );
    })
    .await;
}

// =============================================================================
// Group Permission Tests
// =============================================================================

#[tokio::test]
async fn test_group_permission_grants_access_to_members() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;

        // Create group member
        let (member_id, member_session) = ctx.create_and_login("member").await;

        // Create a group and add member
        let group_id = ctx.create_group(owner_id, "Test Group").await;
        ctx.add_group_member(group_id, member_id).await;

        // Create a private recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "Group Shared Recipe", "private")
            .await;

        // Grant view permission to the group
        let _ = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "group",
                    "subject_id": group_id,
                    "permission_level": "view"
                }),
                &owner_session,
            )
            .await;

        // Group member should be able to view
        let response = ctx
            .get_with_session(&format!("/api/v1/recipes/{}", recipe_id), &member_session)
            .await;

        assert_eq!(
            response.status, 200,
            "Group member should access recipe shared with group"
        );
    })
    .await;
}

#[tokio::test]
async fn test_non_group_member_cannot_access() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;

        // Create non-member
        let (_, non_member_session) = ctx.create_and_login("nonmember").await;

        // Create a group (without adding non-member)
        let group_id = ctx.create_group(owner_id, "Exclusive Group").await;

        // Create a private recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "Group Only Recipe", "private")
            .await;

        // Grant permission only to group
        let _ = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "group",
                    "subject_id": group_id,
                    "permission_level": "view"
                }),
                &owner_session,
            )
            .await;

        // Non-member should NOT be able to view
        let response = ctx
            .get_with_session(
                &format!("/api/v1/recipes/{}", recipe_id),
                &non_member_session,
            )
            .await;

        assert_eq!(
            response.status, 404,
            "Non-group-member should not access group-shared recipe"
        );
    })
    .await;
}

// =============================================================================
// Followers Visibility Tests
// =============================================================================

#[tokio::test]
async fn test_followers_only_accessible_to_followers() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, _) = ctx.create_and_login("owner").await;

        // Create follower
        let (follower_id, follower_session) = ctx.create_and_login("follower").await;

        // Create follow relationship
        ctx.create_follow(follower_id, owner_id).await;

        // Create followers_only recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "Followers Recipe", "followers_only")
            .await;

        // Follower should be able to view
        let response = ctx
            .get_with_session(&format!("/api/v1/recipes/{}", recipe_id), &follower_session)
            .await;

        assert_eq!(
            response.status, 200,
            "Follower should access followers_only recipe"
        );
    })
    .await;
}

#[tokio::test]
async fn test_followers_only_not_accessible_to_non_followers() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, _) = ctx.create_and_login("owner").await;

        // Create non-follower
        let (_, non_follower_session) = ctx.create_and_login("stranger").await;

        // Create followers_only recipe (no follow relationship)
        let recipe_id = ctx
            .create_recipe(owner_id, "Exclusive Recipe", "followers_only")
            .await;

        // Non-follower should NOT be able to view
        let response = ctx
            .get_with_session(
                &format!("/api/v1/recipes/{}", recipe_id),
                &non_follower_session,
            )
            .await;

        assert_eq!(
            response.status, 404,
            "Non-follower should not access followers_only recipe"
        );
    })
    .await;
}

// =============================================================================
// Permission Revocation Tests
// =============================================================================

#[tokio::test]
async fn test_revoked_permission_removes_access() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;

        // Create user
        let (user_id, user_session) = ctx.create_and_login("user").await;

        // Create a private recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "Revokable Recipe", "private")
            .await;

        // Grant permission
        let grant_response = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": user_id,
                    "permission_level": "view"
                }),
                &owner_session,
            )
            .await;

        let perm_id = grant_response
            .body
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap();

        // Verify access works
        let response = ctx
            .get_with_session(&format!("/api/v1/recipes/{}", recipe_id), &user_session)
            .await;
        assert_eq!(
            response.status, 200,
            "User should have access before revoke"
        );

        // Revoke permission
        let revoke_response = ctx
            .delete_with_session(
                &format!("/api/v1/recipes/{}/permissions/{}", recipe_id, perm_id),
                &owner_session,
            )
            .await;
        assert!(revoke_response.is_success(), "Revoke should succeed");

        // Access should now be denied
        let response = ctx
            .get_with_session(&format!("/api/v1/recipes/{}", recipe_id), &user_session)
            .await;
        assert_eq!(
            response.status, 404,
            "User should not have access after revoke"
        );
    })
    .await;
}

// =============================================================================
// Permission Level Hierarchy Tests
// =============================================================================

#[tokio::test]
async fn test_edit_permission_includes_view() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;

        // Create user
        let (user_id, user_session) = ctx.create_and_login("user").await;

        // Create recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "Hierarchy Test", "private")
            .await;

        // Grant only edit (not view explicitly)
        let _ = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": user_id,
                    "permission_level": "edit"
                }),
                &owner_session,
            )
            .await;

        // User should be able to view (edit includes view)
        let response = ctx
            .get_with_session(&format!("/api/v1/recipes/{}", recipe_id), &user_session)
            .await;

        assert_eq!(
            response.status, 200,
            "Edit permission should include view access"
        );
    })
    .await;
}

// =============================================================================
// Non-Owner Cannot Grant Permissions
// =============================================================================

#[tokio::test]
async fn test_non_owner_cannot_grant_permission() {
    run_test(|mut ctx| async move {
        // Create owner
        let (owner_id, owner_session) = ctx.create_and_login("owner").await;

        // Create non-owner with view access
        let (non_owner_id, non_owner_session) = ctx.create_and_login("nonowner").await;

        // Create third user
        let (third_id, _) = ctx.create_and_login("third").await;

        // Create recipe
        let recipe_id = ctx
            .create_recipe(owner_id, "Permission Test", "private")
            .await;

        // Grant edit to non-owner
        let _ = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": non_owner_id,
                    "permission_level": "edit"
                }),
                &owner_session,
            )
            .await;

        // Non-owner tries to grant permission to third user (should fail)
        let response = ctx
            .post_with_session(
                &format!("/api/v1/recipes/{}/permissions", recipe_id),
                serde_json::json!({
                    "subject_type": "user",
                    "subject_id": third_id,
                    "permission_level": "view"
                }),
                &non_owner_session,
            )
            .await;

        assert_eq!(
            response.status, 404,
            "Non-owner should not be able to grant permissions"
        );
    })
    .await;
}
