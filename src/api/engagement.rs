use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use uuid::Uuid;

use crate::api::middleware::{AuthUser, OptionalAuthUser};
use crate::core::error::AppResult;
use crate::models::{Comment, CommentWithAuthor, CreateComment, RatingSummary, SetRatingRequest};
use crate::services::{CommentService, RatingService, RecipeService};
use crate::AppState;

/// Comment + rating routes, nested under `/api/v1/recipes`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{id}/comments", get(list_comments).post(add_comment))
        .route(
            "/{id}/comments/{comment_id}",
            axum::routing::delete(delete_comment),
        )
        .route(
            "/{id}/rating",
            put(set_rating).get(get_rating).delete(delete_rating),
        )
}

/// GET /api/v1/recipes/{id}/comments
async fn list_comments(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: OptionalAuthUser,
) -> AppResult<Json<Vec<CommentWithAuthor>>> {
    // Enforce view permission (404 if not allowed).
    let viewer_id = auth.0.as_ref().map(|u| u.id);
    RecipeService::get_by_id_authorized(&state.db, id, viewer_id).await?;

    let comments = CommentService::list_comments(&state.db, id).await?;
    Ok(Json(comments))
}

/// POST /api/v1/recipes/{id}/comments
async fn add_comment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(input): Json<CreateComment>,
) -> AppResult<(StatusCode, Json<Comment>)> {
    // Must be able to view the recipe to comment on it.
    RecipeService::get_by_id_authorized(&state.db, id, Some(auth.id)).await?;

    let comment = CommentService::add_comment(&state.db, id, auth.id, &input.body).await?;
    Ok((StatusCode::CREATED, Json(comment)))
}

/// DELETE /api/v1/recipes/{id}/comments/{comment_id}
async fn delete_comment(
    State(state): State<AppState>,
    Path((id, comment_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    CommentService::delete_comment(&state.db, id, comment_id, auth.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/recipes/{id}/rating
async fn get_rating(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: OptionalAuthUser,
) -> AppResult<Json<RatingSummary>> {
    let viewer_id = auth.0.as_ref().map(|u| u.id);
    RecipeService::get_by_id_authorized(&state.db, id, viewer_id).await?;

    let summary = RatingService::get_summary(&state.db, id, viewer_id).await?;
    Ok(Json(summary))
}

/// PUT /api/v1/recipes/{id}/rating
async fn set_rating(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(input): Json<SetRatingRequest>,
) -> AppResult<Json<RatingSummary>> {
    RecipeService::get_by_id_authorized(&state.db, id, Some(auth.id)).await?;

    RatingService::set_rating(&state.db, id, auth.id, input.value).await?;
    let summary = RatingService::get_summary(&state.db, id, Some(auth.id)).await?;
    Ok(Json(summary))
}

/// DELETE /api/v1/recipes/{id}/rating
async fn delete_rating(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<Json<RatingSummary>> {
    RatingService::delete_rating(&state.db, id, auth.id).await?;
    let summary = RatingService::get_summary(&state.db, id, Some(auth.id)).await?;
    Ok(Json(summary))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_are_configured() {
        let _ = routes();
    }
}
