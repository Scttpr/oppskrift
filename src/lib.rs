//! Oppskrift - A federated recipe sharing platform
//!
//! This library crate exposes the core functionality for use by the binary
//! and integration tests.

use axum::{extract::ConnectInfo, middleware, routing::get, Router};
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

use crate::api::middleware::security_headers;
use crate::core::request_id::request_id_middleware;

pub mod api;
pub mod core;
pub mod handlers;
pub mod jobs;
pub mod models;
pub mod services;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

/// Create the application router (exposed for testing)
/// For production, use with `into_make_service_with_connect_info::<SocketAddr>()`
pub fn app_router(state: AppState) -> Router {
    create_router(state)
}

/// Create the application router for testing
/// Adds a mock ConnectInfo layer for tests that don't have real socket connections
pub fn test_app_router(state: AppState) -> Router {
    create_router(state).layer(axum::Extension(ConnectInfo(SocketAddr::from((
        [127, 0, 0, 1],
        0,
    )))))
}

/// Create the application router with all middleware
fn create_router(state: AppState) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // API routes
        .nest("/api/v1", api::routes())
        // ActivityPub federation routes
        .merge(api::federation_routes())
        // Content syndication routes (RSS, Atom, oEmbed)
        .merge(api::syndication_routes())
        // API documentation
        .merge(api::docs_routes())
        // HTML handler routes
        .merge(handlers::routes())
        // Static file serving
        .nest_service("/static", ServeDir::new("static"))
        .layer(middleware::from_fn(security_headers))
        .layer(middleware::from_fn(request_id_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that AppState can be created and cloned
    #[test]
    fn test_app_state_is_clone() {
        // This is a compile-time check - AppState must implement Clone
        fn assert_clone<T: Clone>() {}
        assert_clone::<AppState>();
    }

    /// Test that the router modules are properly configured
    /// This test doesn't need a database - it just verifies the routing
    /// configuration doesn't panic during setup
    #[test]
    fn test_api_routes_compile() {
        // These calls verify all route configurations compile correctly
        let _ = api::routes();
        let _ = api::federation_routes();
        let _ = api::syndication_routes();
        let _ = api::docs_routes();
        let _ = handlers::routes();
    }
}
