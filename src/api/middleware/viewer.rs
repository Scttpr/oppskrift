//! Viewer extractor — the signed-in user, loaded once.
//!
//! Builds on [`AuthUser`]: validates the session, then loads the `User` row.
//! Replaces the repeated `UserService::get_by_id(&state.db, auth.id)` pattern
//! in HTML page handlers, carrying the session identity (`id`, `session_id`)
//! alongside the loaded `user` so a single extractor covers both needs.

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use super::AuthUser;
use crate::models::User;
use crate::services::UserService;

/// The authenticated user with their loaded profile row.
#[derive(Debug, Clone)]
pub struct Viewer {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user: User,
}

/// Extract the signed-in user and their profile in one crossing.
///
/// Fails with `AuthUser`'s rejection (401) when unauthenticated, or maps the
/// `get_by_id` error (404) when the session points at a deleted user.
impl FromRequestParts<crate::AppState> for Viewer {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;
        let user = UserService::get_by_id(&state.db, auth.id)
            .await
            .map_err(IntoResponse::into_response)?;
        Ok(Viewer {
            id: auth.id,
            session_id: auth.session_id,
            user,
        })
    }
}

/// Optional viewer — `None` for anonymous visitors or a deleted user.
#[derive(Debug, Clone)]
pub struct OptionalViewer(pub Option<Viewer>);

impl FromRequestParts<crate::AppState> for OptionalViewer {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        Ok(OptionalViewer(
            Viewer::from_request_parts(parts, state).await.ok(),
        ))
    }
}
