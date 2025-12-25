use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::get,
    Router,
};
use uuid::Uuid;

use crate::api::middleware::OptionalAuthUser;
use crate::lib::error::AppResult;
use crate::lib::pagination::{PaginationMeta, PaginationParams};
use crate::models::{RecipeSummary, UserProfile};
use crate::services::{RecipeService, UserService};
use crate::AppState;

/// User page routes
pub fn routes() -> Router<AppState> {
    Router::new().route("/{id}", get(user_profile_page))
}

/// User profile page template
#[derive(Template)]
#[template(path = "users/profile.html")]
struct UserProfileTemplate {
    profile: UserProfile,
    recipes: Vec<RecipeSummary>,
    pagination: PaginationMeta,
    is_own_profile: bool,
}

/// User profile page handler
async fn user_profile_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, id).await?;
    let profile = UserProfile::from(user);

    let recipes_page = RecipeService::list_by_author(&state.db, id, &params).await?;

    let is_own_profile = auth.0.as_ref().map(|u| u.id) == Some(id);

    let template = UserProfileTemplate {
        profile,
        recipes: recipes_page.data,
        pagination: recipes_page.pagination,
        is_own_profile,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}
