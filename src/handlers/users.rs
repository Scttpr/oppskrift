use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    response::Html,
    routing::get,
};
use uuid::Uuid;

use crate::AppState;
use crate::api::middleware::{AuthUser, OptionalAuthUser};
use crate::lib::error::AppResult;
use crate::lib::pagination::{PaginationMeta, PaginationParams};
use crate::models::{FollowCounts, RecipeSummary, UserProfile};
use crate::services::{FollowService, RecipeService, SavedRecipeService, UserService};

/// User page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{id}", get(user_profile_page))
        .route("/{id}/saved", get(saved_recipes_page))
}

/// User profile page template
#[derive(Template)]
#[template(path = "users/profile.html")]
struct UserProfileTemplate {
    profile: UserProfile,
    recipes: Vec<RecipeSummary>,
    pagination: PaginationMeta,
    is_own_profile: bool,
    follow_counts: FollowCounts,
    is_following: bool,
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
    let follow_counts = FollowService::get_counts(&state.db, id).await?;
    let is_following = if let Some(ref current_user) = auth.0 {
        FollowService::is_following(&state.db, current_user.id, id).await?
    } else {
        false
    };

    let template = UserProfileTemplate {
        profile,
        recipes: recipes_page.data,
        pagination: recipes_page.pagination,
        is_own_profile,
        follow_counts,
        is_following,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Saved recipes page template
#[derive(Template)]
#[template(path = "users/saved.html")]
struct SavedRecipesTemplate {
    recipes: Vec<RecipeSummary>,
    pagination: PaginationMeta,
}

/// Saved recipes page handler (requires auth)
async fn saved_recipes_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    // Only allow viewing own saved recipes
    if auth.id != id {
        return Err(crate::lib::error::AppError::Forbidden(
            "You can only view your own saved recipes".to_string(),
        ));
    }

    let saved_page = SavedRecipeService::get_saved(&state.db, auth.id, &params).await?;

    let template = SavedRecipesTemplate {
        recipes: saved_page.data,
        pagination: saved_page.pagination,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}
