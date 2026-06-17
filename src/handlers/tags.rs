use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::get,
    Router,
};

use crate::core::error::AppResult;
use crate::core::pagination::{PaginationMeta, PaginationParams};
use crate::models::{RecipeSummary, TagWithCount};
use crate::services::TagService;
use crate::AppState;

/// Tag page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tags_page))
        .route("/{slug}", get(tag_recipes_page))
}

/// Tag index template
#[derive(Template)]
#[template(path = "tags/index.html")]
struct TagIndexTemplate {
    tags: Vec<TagWithCount>,
}

/// Tag index page handler — browse all tags
async fn list_tags_page(State(state): State<AppState>) -> AppResult<Html<String>> {
    let tags = TagService::list_with_counts(&state.db).await?;
    let template = TagIndexTemplate { tags };
    crate::core::render(&template)
}

/// Recipes-by-tag template
#[derive(Template)]
#[template(path = "tags/recipes.html")]
struct TagRecipesTemplate {
    tag_name: String,
    recipes: Vec<RecipeSummary>,
    pagination: PaginationMeta,
}

/// Recipes-by-tag page handler
async fn tag_recipes_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Html<String>> {
    let tag = TagService::get_by_slug(&state.db, &slug).await?;
    let results = TagService::list_public_recipes_by_tag(&state.db, &slug, &params).await?;

    let template = TagRecipesTemplate {
        tag_name: tag.name,
        recipes: results.data,
        pagination: results.pagination,
    };
    crate::core::render(&template)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_returns_router() {
        let _ = routes();
    }

    #[test]
    fn test_tag_index_template_renders() {
        let template = TagIndexTemplate { tags: vec![] };
        assert!(template.render().is_ok());
    }
}
