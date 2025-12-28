use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
    routing::get,
    Router,
};

use crate::api::middleware::AuthUser;
use crate::core::error::AppResult;
use crate::core::pagination::{PaginationMeta, PaginationParams};
use crate::models::ActivityWithActor;
use crate::services::ActivityService;
use crate::AppState;

/// Feed routes
pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(feed_page))
}

/// Feed page template
#[derive(Template)]
#[template(path = "feed/index.html")]
struct FeedTemplate {
    activities: Vec<ActivityWithActor>,
    pagination: PaginationMeta,
}

/// Feed page handler
async fn feed_page(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let feed = ActivityService::get_feed(&state.db, auth.id, &params).await?;

    let template = FeedTemplate {
        activities: feed.data,
        pagination: feed.pagination,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}
