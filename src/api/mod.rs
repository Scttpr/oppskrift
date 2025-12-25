// API module - REST endpoints, ActivityPub handlers

pub mod middleware;
pub mod users;

use axum::Router;

use crate::AppState;

/// Create API v1 routes
pub fn routes() -> Router<AppState> {
    Router::new().nest("/users", users::routes())
}
