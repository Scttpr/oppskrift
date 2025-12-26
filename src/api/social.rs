use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::api::middleware::AuthUser;
use crate::lib::error::AppResult;
use crate::lib::pagination::{PaginatedResponse, PaginationParams};
use crate::models::{Activity, ActivityWithActor, Follow, RecipeSummary, SavedRecipe};
use crate::services::{ActivityService, FollowService, SavedRecipeService};
use crate::AppState;

/// Social feature routes
pub fn routes() -> Router<AppState> {
    Router::new()
        // Follow endpoints
        .route("/users/{id}/follow", post(follow_user))
        .route("/users/{id}/follow", delete(unfollow_user))
        // Save endpoints
        .route("/recipes/{id}/save", post(save_recipe))
        .route("/recipes/{id}/save", delete(unsave_recipe))
        // Saved recipes list
        .route("/users/{id}/saved", get(get_saved_recipes))
        // Share endpoint
        .route("/recipes/{id}/share", post(share_recipe))
        // Feed endpoint
        .route("/feed", get(get_feed))
}

/// Response for follow/unfollow operations
#[derive(Debug, Serialize)]
struct FollowResponse {
    followed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    follow: Option<Follow>,
}

/// Follow a user
async fn follow_user(
    State(state): State<AppState>,
    Path(following_id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<(StatusCode, Json<FollowResponse>)> {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let follow = FollowService::follow(&state.db, auth.id, following_id, &base_url).await?;

    // Create follow activity
    let _ =
        ActivityService::create_follow_activity(&state.db, auth.id, following_id, &base_url).await;

    Ok((
        StatusCode::CREATED,
        Json(FollowResponse {
            followed: true,
            follow: Some(follow),
        }),
    ))
}

/// Unfollow a user
async fn unfollow_user(
    State(state): State<AppState>,
    Path(following_id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<Json<FollowResponse>> {
    FollowService::unfollow(&state.db, auth.id, following_id).await?;

    Ok(Json(FollowResponse {
        followed: false,
        follow: None,
    }))
}

/// Response for save/unsave operations
#[derive(Debug, Serialize)]
struct SaveResponse {
    saved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    saved_recipe: Option<SavedRecipe>,
}

/// Save a recipe
async fn save_recipe(
    State(state): State<AppState>,
    Path(recipe_id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<(StatusCode, Json<SaveResponse>)> {
    let saved_recipe = SavedRecipeService::save(&state.db, auth.id, recipe_id).await?;

    Ok((
        StatusCode::CREATED,
        Json(SaveResponse {
            saved: true,
            saved_recipe: Some(saved_recipe),
        }),
    ))
}

/// Unsave a recipe
async fn unsave_recipe(
    State(state): State<AppState>,
    Path(recipe_id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<Json<SaveResponse>> {
    SavedRecipeService::unsave(&state.db, auth.id, recipe_id).await?;

    Ok(Json(SaveResponse {
        saved: false,
        saved_recipe: None,
    }))
}

/// Get user's saved recipes
async fn get_saved_recipes(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth: AuthUser,
) -> AppResult<Json<PaginatedResponse<RecipeSummary>>> {
    // Users can only view their own saved recipes
    if auth.id != user_id {
        return Err(crate::lib::error::AppError::Forbidden(
            "Cannot view other users' saved recipes".to_string(),
        ));
    }

    let saved = SavedRecipeService::get_saved(&state.db, user_id, &params).await?;
    Ok(Json(saved))
}

/// Response for share operation
#[derive(Debug, Serialize)]
struct ShareResponse {
    shared: bool,
    activity: Activity,
}

/// Share a recipe (creates an Announce activity)
async fn share_recipe(
    State(state): State<AppState>,
    Path(recipe_id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<(StatusCode, Json<ShareResponse>)> {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let activity = ActivityService::share_recipe(&state.db, auth.id, recipe_id, &base_url).await?;

    Ok((
        StatusCode::CREATED,
        Json(ShareResponse {
            shared: true,
            activity,
        }),
    ))
}

/// Get activity feed (activities from followed users)
async fn get_feed(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
    auth: AuthUser,
) -> AppResult<Json<PaginatedResponse<ActivityWithActor>>> {
    let feed = ActivityService::get_feed(&state.db, auth.id, &params).await?;
    Ok(Json(feed))
}
