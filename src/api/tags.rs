use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};

use crate::core::error::AppResult;
use crate::core::pagination::{PaginatedResponse, PaginationParams};
use crate::models::{RecipeSummary, TagWithCount};
use crate::services::TagService;
use crate::AppState;

/// Tag API routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tags))
        .route("/{slug}/recipes", get(list_recipes_by_tag))
}

/// GET /api/v1/tags
/// List all tags that have public recipes, with counts.
async fn list_tags(State(state): State<AppState>) -> AppResult<Json<Vec<TagWithCount>>> {
    let tags = TagService::list_with_counts(&state.db).await?;
    Ok(Json(tags))
}

/// GET /api/v1/tags/{slug}/recipes
/// List public recipes carrying the given tag.
async fn list_recipes_by_tag(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<RecipeSummary>>> {
    let recipes = TagService::list_public_recipes_by_tag(&state.db, &slug, &params).await?;
    Ok(Json(recipes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_are_configured() {
        let _ = routes();
    }
}
