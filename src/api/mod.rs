// API module - REST endpoints, ActivityPub handlers

pub mod activitypub;
pub mod books;
pub mod middleware;
pub mod recipes;
pub mod social;
pub mod users;
pub mod webfinger;

use axum::Router;

use crate::AppState;

/// Create API v1 routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/users", users::routes())
        .nest("/recipes", recipes::routes())
        .nest("/books", books::routes())
        .merge(social::routes())
}

/// Create ActivityPub federation routes
pub fn federation_routes() -> Router<AppState> {
    Router::new()
        .merge(webfinger::routes())
        .nest("/ap", activitypub::routes())
}
