// Handlers module - HTML page handlers (serve templates)

pub mod auth;
pub mod books;
pub mod feed;
pub mod legal;
pub mod recipes;
pub mod settings;
pub mod users;

use askama::Template;
use axum::{extract::State, response::Html, routing::get, Router};

use crate::api::middleware::OptionalAuthUser;
use crate::core::error::{AppError, AppResult};
use crate::core::pagination::PaginationParams;
use crate::models::RecipeSummary;
use crate::services::{RecipeService, UserService};
use crate::AppState;

/// Create HTML page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(home_page))
        .nest("/recipes", recipes::routes())
        .nest("/books", books::routes())
        .nest("/users", users::routes())
        .nest("/feed", feed::routes())
        .nest("/settings", settings::routes())
        .merge(auth::routes())
        .merge(legal::routes())
}

/// Simple user info for templates
pub struct CurrentUser {
    pub id: uuid::Uuid,
    pub display_name: String,
}

/// Home page template
#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    recent_recipes: Vec<RecipeSummary>,
    current_user: Option<CurrentUser>,
}

/// Home page handler
async fn home_page(
    State(state): State<AppState>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let params = PaginationParams {
        page: 1,
        page_size: 6,
    };
    let recipes_page = RecipeService::list_public(&state.db, &params).await?;

    // Get current user if authenticated
    let current_user = if let Some(auth_user) = auth.0 {
        UserService::get_by_id(&state.db, auth_user.id)
            .await
            .ok()
            .map(|u| {
                let display_name = if u.display_name.is_empty() {
                    u.username
                } else {
                    u.display_name
                };
                CurrentUser {
                    id: u.id,
                    display_name,
                }
            })
    } else {
        None
    };

    let template = HomeTemplate {
        recent_recipes: recipes_page.data,
        current_user,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}
