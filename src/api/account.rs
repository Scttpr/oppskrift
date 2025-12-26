//! Account API endpoints
//!
//! Provides endpoints for authenticated users to manage their account.
//! All endpoints require a valid session.

use axum::{extract::State, routing::get, Json, Router};

use crate::api::middleware::AuthUser;
use crate::lib::error::AppError;
use crate::models::UserProfile;
use crate::services::UserService;
use crate::AppState;

/// Account routes - all require authentication
pub fn routes() -> Router<AppState> {
    Router::new().route("/profile", get(get_profile))
}

/// GET /api/account/profile
///
/// Get the authenticated user's profile.
///
/// ## Response
/// - `200 OK`: User profile
/// - `401 Unauthorized`: Not authenticated
async fn get_profile(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<UserProfile>, AppError> {
    let user = UserService::get_by_id(&state.db, auth_user.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %auth_user.id, "Failed to get user profile");
            AppError::Internal("Failed to get profile".to_string())
        })?;

    Ok(Json(UserProfile::from(user)))
}
