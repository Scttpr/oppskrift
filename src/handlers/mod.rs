// Handlers module - HTML page handlers (serve templates)

pub mod books;
pub mod feed;
pub mod legal;
pub mod recipes;
pub mod users;

use axum::Router;

use crate::AppState;

/// Create HTML page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/recipes", recipes::routes())
        .nest("/books", books::routes())
        .nest("/users", users::routes())
        .nest("/feed", feed::routes())
        .merge(legal::routes())
}
