//! Template rendering helpers

use crate::core::error::{AppError, AppResult};
use axum::response::Html;

/// Render an Askama template into an HTML response.
///
/// Centralizes the `Template error: ...` mapping used across handlers.
pub fn render<T: askama::Template>(t: &T) -> AppResult<Html<String>> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Internal(format!("Template error: {e}")))
}
