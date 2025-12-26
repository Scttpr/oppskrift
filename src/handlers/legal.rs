use askama::Template;
use axum::{response::Html, routing::get, Router};

use crate::lib::error::{AppError, AppResult};
use crate::AppState;

/// Legal page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/privacy", get(privacy_page))
        .route("/terms", get(terms_page))
}

/// Privacy policy template
#[derive(Template)]
#[template(path = "legal/privacy.html")]
struct PrivacyTemplate;

/// Terms of service template
#[derive(Template)]
#[template(path = "legal/terms.html")]
struct TermsTemplate;

/// Privacy policy page handler
async fn privacy_page() -> AppResult<Html<String>> {
    let template = PrivacyTemplate;
    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Terms of service page handler
async fn terms_page() -> AppResult<Html<String>> {
    let template = TermsTemplate;
    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}
