//! Groups API endpoints
//!
//! Provides CRUD operations for groups and member management.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::middleware::AuthUser;
use crate::core::error::AppResult;
use crate::models::{
    AddMemberRequest, CreateGroupRequest, Group, GroupDetail, GroupFilter, GroupListResponse,
    UpdateGroupRequest,
};
use crate::services::GroupService;
use crate::AppState;

/// Groups API routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_groups).post(create_group))
        .route(
            "/{id}",
            get(get_group).put(update_group).delete(delete_group),
        )
        .route("/{id}/members", post(add_member))
        .route("/{id}/members/{user_id}", delete(remove_member))
        .route("/{id}/leave", post(leave_group))
}

/// Query parameters for listing groups
#[derive(Debug, Deserialize)]
pub struct ListGroupsParams {
    #[serde(default)]
    pub filter: GroupFilter,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    20
}

/// POST /api/v1/groups
/// Create a new group
async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(request): Json<CreateGroupRequest>,
) -> AppResult<(StatusCode, Json<Group>)> {
    let group = GroupService::create(&state.db, auth.id, request).await?;
    Ok((StatusCode::CREATED, Json(group)))
}

/// GET /api/v1/groups
/// List groups the user is a member of
async fn list_groups(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListGroupsParams>,
) -> AppResult<Json<GroupListResponse>> {
    let page_size = params.page_size.clamp(1, 100);
    let page = params.page.max(1);

    let (groups, total) =
        GroupService::list_for_user(&state.db, auth.id, params.filter, page, page_size).await?;

    Ok(Json(GroupListResponse {
        groups,
        page,
        page_size,
        total,
    }))
}

/// GET /api/v1/groups/{id}
/// Get a group by ID with members
async fn get_group(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<Json<GroupDetail>> {
    // Verify user is a member
    if !GroupService::is_member(&state.db, id, auth.id).await? {
        return Err(crate::core::error::AppError::NotFound(
            "Group not found".to_string(),
        ));
    }

    let detail = GroupService::get_detail(&state.db, id, auth.id).await?;
    Ok(Json(detail))
}

/// PUT /api/v1/groups/{id}
/// Update a group (owner only)
async fn update_group(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(request): Json<UpdateGroupRequest>,
) -> AppResult<Json<Group>> {
    let group = GroupService::update(&state.db, id, auth.id, request).await?;
    Ok(Json(group))
}

/// DELETE /api/v1/groups/{id}
/// Delete a group (owner only)
async fn delete_group(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    GroupService::delete(&state.db, id, auth.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/groups/{id}/members
/// Add a member to a group (owner only)
async fn add_member(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(request): Json<AddMemberRequest>,
) -> AppResult<StatusCode> {
    GroupService::add_member(&state.db, id, auth.id, request).await?;
    Ok(StatusCode::CREATED)
}

/// DELETE /api/v1/groups/{id}/members/{user_id}
/// Remove a member from a group (owner only)
async fn remove_member(
    State(state): State<AppState>,
    Path((id, user_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    GroupService::remove_member(&state.db, id, auth.id, user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/groups/{id}/leave
/// Leave a group (any member except owner)
async fn leave_group(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    GroupService::leave(&state.db, id, auth.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_are_configured() {
        let _router = routes();
    }

    #[test]
    fn test_default_page() {
        assert_eq!(default_page(), 1);
    }

    #[test]
    fn test_default_page_size() {
        assert_eq!(default_page_size(), 20);
    }
}
