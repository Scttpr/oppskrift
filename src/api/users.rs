use axum::{
    extract::{Path, State},
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::middleware::AuthUser;
use crate::lib::error::AppResult;
use crate::lib::pagination::PaginationParams;
use crate::models::user::{UpdateUser, User, UserProfile};
use crate::models::{RecipeSummary, Book};
use crate::services::{BookService, FollowService, RecipeService, SavedRecipeService, UserService};
use crate::AppState;

/// User API routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_current_user).patch(update_current_user))
        .route("/me/export", get(export_user_data))
        .route("/me/federation", patch(toggle_federation))
        .route("/{id}", get(get_user_by_id))
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
async fn get_current_user(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<User>> {
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
    pub books: Vec<Book>,
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
    let params = PaginationParams { page: 1, page_size: 1000 };
    let recipes_result = RecipeService::list_by_author(&state.db, auth.id, &params).await?;

    // Get user's books
    let books = BookService::list_by_owner(&state.db, auth.id, &params).await?;

    // Get follow counts
    let follow_counts = FollowService::get_counts(&state.db, auth.id).await?;

    // Get saved recipes count
    let saved_recipes = SavedRecipeService::get_saved(&state.db, auth.id, &params).await?;

    let export = UserDataExport {
        exported_at: chrono::Utc::now(),
        profile: user,
        recipes: recipes_result.data,
        books: books.data,
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
