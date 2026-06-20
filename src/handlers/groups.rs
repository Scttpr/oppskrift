//! Groups page handlers
//!
//! Provides HTML pages for group management.

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::get,
    Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::middleware::Viewer;
use crate::core::error::{AppError, AppResult};
use crate::models::{GroupDetail, GroupFilter, GroupWithMeta, User};
use crate::services::GroupService;
use crate::AppState;

/// Group page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_groups_page))
        .route("/new", get(new_group_page))
        .route("/{id}", get(view_group_page))
        .route("/{id}/edit", get(edit_group_page))
}

/// Query parameters for group list
#[derive(Debug, Deserialize)]
pub struct GroupListParams {
    #[serde(default)]
    pub filter: GroupFilter,
    #[serde(default = "default_page")]
    pub page: i64,
}

fn default_page() -> i64 {
    1
}

/// Group list page template
#[derive(Template)]
#[template(path = "groups/list.html")]
#[allow(dead_code)] // Fields available for template use
struct GroupListTemplate {
    groups: Vec<GroupWithMeta>,
    filter: GroupFilter,
    page: i64,
    total_pages: i64,
    total: i64,
    user: User,
}

/// Group list page handler
async fn list_groups_page(
    State(state): State<AppState>,
    viewer: Viewer,
    Query(params): Query<GroupListParams>,
) -> AppResult<Html<String>> {
    let user = viewer.user;
    let page_size = 20i64;
    let (groups, total) =
        GroupService::list_for_user(&state.db, viewer.id, params.filter, params.page, page_size)
            .await?;

    let total_pages = (total + page_size - 1) / page_size;

    let template = GroupListTemplate {
        groups,
        filter: params.filter,
        page: params.page,
        total_pages,
        total,
        user,
    };

    crate::core::render(&template)
}

/// New group form template
#[derive(Template)]
#[template(path = "groups/form.html")]
#[allow(dead_code)] // Fields available for template use
struct NewGroupTemplate {
    group: Option<GroupWithMeta>,
    user: User,
}

/// New group page handler
async fn new_group_page(viewer: Viewer) -> AppResult<Html<String>> {
    let user = viewer.user;

    let template = NewGroupTemplate { group: None, user };

    crate::core::render(&template)
}

/// Group view page template
#[derive(Template)]
#[template(path = "groups/view.html")]
#[allow(dead_code)] // Fields available for template use
struct GroupViewTemplate {
    group: GroupDetail,
    user: User,
}

/// View group page handler
async fn view_group_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    viewer: Viewer,
) -> AppResult<Html<String>> {
    let user = viewer.user;

    // Check membership
    if !GroupService::is_member(&state.db, id, viewer.id).await? {
        return Err(AppError::NotFound("Groupe introuvable".to_string()));
    }

    let group = GroupService::get_detail(&state.db, id, viewer.id).await?;

    let template = GroupViewTemplate { group, user };

    crate::core::render(&template)
}

/// Edit group form template
#[derive(Template)]
#[template(path = "groups/form.html")]
#[allow(dead_code)] // Fields available for template use
struct EditGroupTemplate {
    group: Option<GroupWithMeta>,
    user: User,
}

/// Edit group page handler
async fn edit_group_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    viewer: Viewer,
) -> AppResult<Html<String>> {
    let user = viewer.user;
    let group = GroupService::get_with_meta(&state.db, id, viewer.id).await?;

    // Only owner can edit
    if !group.is_owner {
        return Err(AppError::NotFound("Groupe introuvable".to_string()));
    }

    let template = EditGroupTemplate {
        group: Some(group),
        user,
    };

    crate::core::render(&template)
}
