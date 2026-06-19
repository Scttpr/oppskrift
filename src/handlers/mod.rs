// Handlers module - HTML page handlers (serve templates)

pub mod auth;
pub mod books;
pub mod feed;
pub mod groups;
pub mod legal;
pub mod permissions;
pub mod recipes;
pub mod settings;
pub mod tags;
pub mod users;

use askama::Template;
use axum::{extract::State, response::Html, routing::get, Router};

use crate::api::middleware::OptionalAuthUser;
use crate::core::error::AppResult;
use crate::core::pagination::PaginationParams;
use crate::models::RecipeSummary;
use crate::services::{RecipeService, UserService};
use crate::AppState;

/// Create HTML page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(home_page))
        .route("/partials/user-menu", get(user_menu_partial))
        .nest("/recipes", recipes::routes())
        .nest("/books", books::routes())
        .nest("/users", users::routes())
        .nest("/groups", groups::routes())
        .nest("/feed", feed::routes())
        .nest("/tags", tags::routes())
        .nest("/settings", settings::routes())
        .merge(auth::routes())
        .merge(legal::routes())
        .merge(permissions::routes())
}

/// Simple user info for templates
pub struct CurrentUser {
    pub id: uuid::Uuid,
    pub display_name: String,
}

/// User menu partial template
#[derive(Template)]
#[template(path = "partials/user_menu.html")]
struct UserMenuTemplate {
    current_user: Option<CurrentUser>,
}

/// User menu partial handler - returns HTML fragment for HTMX
async fn user_menu_partial(
    State(state): State<AppState>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
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

    let template = UserMenuTemplate { current_user };

    crate::core::render(&template)
}

/// Home page template
#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    recent_recipes: Vec<RecipeSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_home_page_is_french() {
        let html = HomeTemplate {
            recent_recipes: vec![],
        }
        .render()
        .expect("home template renders");

        assert!(html.contains("lang=\"fr\""), "html must declare lang=fr");
        assert!(
            !html.contains("lang=\"en\""),
            "html must not declare lang=en"
        );

        for english in [
            "Recipes",
            "Search",
            "Books",
            "Tags",
            "Feed",
            "Sign In",
            "About",
            "Privacy",
            "Terms",
            "Federated",
            "Cook, share",
            "Create Account",
        ] {
            assert!(
                !html.contains(english),
                "home page should not contain English sentinel: {english:?}"
            );
        }
    }

    #[test]
    fn test_user_menu_is_french() {
        let logged_in = UserMenuTemplate {
            current_user: Some(CurrentUser {
                id: uuid::Uuid::nil(),
                display_name: "Alice".to_string(),
            }),
        }
        .render()
        .expect("user menu renders");
        for english in ["My Profile", "Settings", "Sign Out", "Open user menu"] {
            assert!(
                !logged_in.contains(english),
                "user menu should not contain English sentinel: {english:?}"
            );
        }

        let logged_out = UserMenuTemplate { current_user: None }
            .render()
            .expect("user menu renders");
        assert!(!logged_out.contains("Sign In"), "login link must be French");
    }
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

    crate::core::render(&template)
}
