//! Permission management page handlers
//!
//! Provides HTML pages for managing resource permissions (sharing).

use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
    routing::get,
    Router,
};
use uuid::Uuid;

use crate::api::middleware::AuthUser;
use crate::core::error::{AppError, AppResult};
use crate::models::{GroupWithMeta, PermissionWithDisplay, ResourceType, User};
use crate::services::{BookService, GroupService, PermissionService, RecipeService, UserService};
use crate::AppState;

/// Permission management routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/recipes/{id}/share", get(recipe_share_page))
        .route("/books/{id}/share", get(book_share_page))
}

/// Recipe share page template
#[derive(Template)]
#[template(path = "permissions/manage.html")]
#[allow(dead_code)] // Fields available for template use
struct RecipeShareTemplate {
    resource_type: String,
    resource_id: Uuid,
    resource_title: String,
    permissions: Vec<PermissionWithDisplay>,
    user_groups: Vec<GroupWithMeta>,
    user: User,
}

/// Recipe share page handler
async fn recipe_share_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    // Verify user owns this recipe
    if !PermissionService::is_owner(&state.db, auth.id, ResourceType::Recipe, id).await? {
        return Err(AppError::NotFound("Recipe not found".to_string()));
    }

    let recipe = RecipeService::get_by_id(&state.db, id).await?;
    let permissions =
        PermissionService::list_permissions(&state.db, auth.id, ResourceType::Recipe, id).await?;

    // Get user's groups for sharing options
    let (user_groups, _) = GroupService::list_for_user(
        &state.db,
        auth.id,
        crate::models::GroupFilter::Owned,
        1,
        100,
    )
    .await?;

    let template = RecipeShareTemplate {
        resource_type: "recipe".to_string(),
        resource_id: id,
        resource_title: recipe.title,
        permissions,
        user_groups,
        user,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Book share page template
#[derive(Template)]
#[template(path = "permissions/manage.html")]
#[allow(dead_code)] // Fields available for template use
struct BookShareTemplate {
    resource_type: String,
    resource_id: Uuid,
    resource_title: String,
    permissions: Vec<PermissionWithDisplay>,
    user_groups: Vec<GroupWithMeta>,
    user: User,
}

/// Book share page handler
async fn book_share_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    // Verify user owns this book
    if !PermissionService::is_owner(&state.db, auth.id, ResourceType::Book, id).await? {
        return Err(AppError::NotFound("Book not found".to_string()));
    }

    let book = BookService::get_by_id(&state.db, id).await?;
    let permissions =
        PermissionService::list_permissions(&state.db, auth.id, ResourceType::Book, id).await?;

    // Get user's groups for sharing options
    let (user_groups, _) = GroupService::list_for_user(
        &state.db,
        auth.id,
        crate::models::GroupFilter::Owned,
        1,
        100,
    )
    .await?;

    let template = BookShareTemplate {
        resource_type: "book".to_string(),
        resource_id: id,
        resource_title: book.title,
        permissions,
        user_groups,
        user,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}
