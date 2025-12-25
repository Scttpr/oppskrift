use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use uuid::Uuid;

use crate::api::middleware::AuthUser;
use crate::lib::error::AppResult;
use crate::models::user::{User, UserProfile};
use crate::services::UserService;
use crate::AppState;

/// User API routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_current_user))
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
