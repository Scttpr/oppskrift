// Handlers module - HTML page handlers (serve templates)

pub mod auth;
pub mod books;
pub mod feed;
pub mod legal;
pub mod recipes;
pub mod users;

use askama::Template;
use axum::{extract::State, response::Html, routing::get, Router};

use crate::lib::error::{AppError, AppResult};
use crate::lib::pagination::PaginationParams;
use crate::models::RecipeSummary;
use crate::services::RecipeService;
use crate::AppState;

/// Create HTML page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(home_page))
        .nest("/recipes", recipes::routes())
        .nest("/books", books::routes())
        .nest("/users", users::routes())
        .nest("/feed", feed::routes())
        .merge(auth::routes())
        .merge(legal::routes())
}

/// Home page template
#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    recent_recipes: Vec<RecipeSummary>,
}

/// Home page handler
async fn home_page(State(state): State<AppState>) -> AppResult<Html<String>> {
    let params = PaginationParams {
        page: 1,
        page_size: 6,
    };
    let recipes_page = RecipeService::list_public(&state.db, &params).await?;

    let template = HomeTemplate {
        recent_recipes: recipes_page.data,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}
