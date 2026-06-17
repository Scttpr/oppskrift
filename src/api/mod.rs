// API module - REST endpoints, ActivityPub handlers

pub mod account;
pub mod activitypub;
pub mod auth;
pub mod books;
pub mod engagement;
pub mod feeds;
pub mod groups;
pub mod middleware;
pub mod oembed;
pub mod openapi;
pub mod recipes;
pub mod social;
pub mod users;
pub mod webfinger;

use axum::Router;

use crate::api::middleware::RateLimiterState;
use crate::AppState;

/// Create API v1 routes (without rate limiting, for tests)
pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/account", account::routes())
        .nest("/users", users::routes())
        .nest("/recipes", recipes::routes())
        .nest("/books", books::routes())
        .nest("/groups", groups::routes())
        .merge(social::routes())
}

/// Create API v1 routes with rate limiting
pub fn routes_with_rate_limit(rate_limiter: RateLimiterState) -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes_with_rate_limit(rate_limiter.clone()))
        .nest("/account", account::routes())
        .nest(
            "/users",
            users::routes_with_rate_limit(rate_limiter.clone()),
        )
        .nest("/recipes", recipes::routes_with_rate_limit(rate_limiter))
        .nest("/books", books::routes())
        .nest("/groups", groups::routes())
        .merge(social::routes())
}

/// Create ActivityPub federation routes
pub fn federation_routes() -> Router<AppState> {
    Router::new()
        .merge(webfinger::routes())
        .nest("/ap", activitypub::routes())
}

/// Create content syndication routes (RSS, Atom, oEmbed)
pub fn syndication_routes() -> Router<AppState> {
    Router::new().merge(feeds::routes()).merge(oembed::routes())
}

/// Create documentation routes
pub fn docs_routes() -> Router<AppState> {
    Router::new().merge(openapi::routes())
}
