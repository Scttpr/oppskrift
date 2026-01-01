use axum::{
    extract::{Path, Query, State},
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::middleware::AuthUser;
use crate::core::error::AppResult;
use crate::core::pagination::{PaginatedResponse, PaginationParams};
use crate::models::user::{UpdateUser, User, UserProfile};
use crate::models::{RecipeBookSummary, RecipeSummary};
use crate::services::{BookService, FollowService, RecipeService, SavedRecipeService, UserService};
use crate::AppState;

/// User API routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_current_user).patch(update_current_user))
        .route("/me/export", get(export_user_data))
        .route("/me/federation", patch(toggle_federation))
        .route("/search", get(search_users))
        .route("/{id}", get(get_user_by_id))
        .route("/{id}/followers", get(get_user_followers))
        .route("/{id}/following", get(get_user_following))
        .route("/{id}/books", get(get_user_books))
}

/// User search query parameters
#[derive(Debug, Deserialize)]
pub struct UserSearchParams {
    /// Username query (prefix match)
    pub q: String,
    /// Max results to return (default 10, max 50)
    #[serde(default = "default_search_limit")]
    pub limit: i64,
}

fn default_search_limit() -> i64 {
    10
}

/// User search result item
#[derive(Debug, Serialize)]
pub struct UserSearchResult {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

/// GET /api/v1/users/search?q=username
/// Search users by username prefix (requires authentication)
async fn search_users(
    State(state): State<AppState>,
    Query(params): Query<UserSearchParams>,
    _auth: AuthUser,
) -> AppResult<Json<Vec<UserSearchResult>>> {
    // Validate query length
    if params.q.is_empty() {
        return Ok(Json(vec![]));
    }

    // Limit results
    let limit = params.limit.clamp(1, 50);

    let users = UserService::search_by_username(&state.db, &params.q, limit).await?;

    let results: Vec<UserSearchResult> = users
        .into_iter()
        .map(|u| UserSearchResult {
            id: u.id,
            username: u.username,
            display_name: u.display_name,
            avatar_url: u.avatar_url,
        })
        .collect();

    Ok(Json(results))
}

/// GET /api/v1/users/{id}
/// Returns public profile for any user
async fn get_user_by_id(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<UserProfile>> {
    let user = UserService::get_by_id(&state.db, id).await?;
    Ok(Json(UserProfile::from(user)))
}

/// GET /api/v1/users/me
/// Returns full user data for authenticated user
async fn get_current_user(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<User>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    Ok(Json(user))
}

/// PATCH /api/v1/users/me
/// Update authenticated user's profile
async fn update_current_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<UpdateUser>,
) -> AppResult<Json<User>> {
    let user = UserService::update(&state.db, auth.id, input).await?;
    Ok(Json(user))
}

/// GDPR data export response
#[derive(Debug, Serialize)]
pub struct UserDataExport {
    pub exported_at: chrono::DateTime<chrono::Utc>,
    pub profile: User,
    pub recipes: Vec<RecipeSummary>,
    pub books: Vec<RecipeBookSummary>,
    pub followers_count: i64,
    pub following_count: i64,
    pub saved_recipes_count: i64,
}

/// GET /api/v1/users/me/export
/// Export all user data for GDPR compliance
async fn export_user_data(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<UserDataExport>> {
    // Get user profile
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    // Get user's recipes (all pages)
    let params = PaginationParams {
        page: 1,
        page_size: 1000,
    };
    let recipes_result = RecipeService::list_by_author(&state.db, auth.id, &params).await?;

    // Get user's books
    let books = BookService::list_by_owner(&state.db, auth.id).await?;

    // Get follow counts
    let follow_counts = FollowService::get_counts(&state.db, auth.id).await?;

    // Get saved recipes count
    let saved_recipes = SavedRecipeService::get_saved(&state.db, auth.id, &params).await?;

    let export = UserDataExport {
        exported_at: chrono::Utc::now(),
        profile: user,
        recipes: recipes_result.data,
        books,
        followers_count: follow_counts.followers_count,
        following_count: follow_counts.following_count,
        saved_recipes_count: saved_recipes.pagination.total_items as i64,
    };

    Ok(Json(export))
}

/// Federation toggle request
#[derive(Debug, Deserialize)]
pub struct FederationToggle {
    pub enabled: bool,
}

/// Federation status response
#[derive(Debug, Serialize)]
pub struct FederationStatus {
    pub federation_enabled: bool,
}

/// PATCH /api/v1/users/me/federation
/// Toggle ActivityPub federation for the current user
async fn toggle_federation(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<FederationToggle>,
) -> AppResult<Json<FederationStatus>> {
    let user = UserService::set_federation_enabled(&state.db, auth.id, input.enabled).await?;

    Ok(Json(FederationStatus {
        federation_enabled: user.federation_enabled,
    }))
}

/// GET /api/v1/users/{id}/followers
/// Get list of users following this user
async fn get_user_followers(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<UserProfile>>> {
    // Verify user exists
    let _ = UserService::get_by_id(&state.db, id).await?;

    let followers = FollowService::get_followers(&state.db, id).await?;
    let profiles: Vec<UserProfile> = followers.into_iter().map(UserProfile::from).collect();

    Ok(Json(profiles))
}

/// GET /api/v1/users/{id}/following
/// Get list of users this user is following
async fn get_user_following(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<UserProfile>>> {
    // Verify user exists
    let _ = UserService::get_by_id(&state.db, id).await?;

    let following = FollowService::get_following(&state.db, id).await?;
    let profiles: Vec<UserProfile> = following.into_iter().map(UserProfile::from).collect();

    Ok(Json(profiles))
}

/// GET /api/v1/users/{id}/books
/// Get user's recipe books with pagination
async fn get_user_books(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<RecipeBookSummary>>> {
    // Verify user exists
    let _ = UserService::get_by_id(&state.db, id).await?;

    let books = BookService::list_by_owner_paginated(&state.db, id, &params).await?;
    Ok(Json(books))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_are_configured() {
        let _router = routes();
    }
}
