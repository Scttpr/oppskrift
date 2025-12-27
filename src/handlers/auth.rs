use askama::Template;
use axum::{response::Html, routing::get, Router};

use crate::lib::error::{AppError, AppResult};
use crate::AppState;

/// Auth page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_page))
        .route("/register", get(register_page))
        .route("/forgot-password", get(forgot_password_page))
}

/// Login page template
#[derive(Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate;

/// Register page template
#[derive(Template)]
#[template(path = "auth/register.html")]
struct RegisterTemplate;

/// Forgot password page template
#[derive(Template)]
#[template(path = "auth/forgot_password.html")]
struct ForgotPasswordTemplate;

/// Login page handler
async fn login_page() -> AppResult<Html<String>> {
    let template = LoginTemplate;
    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Register page handler
async fn register_page() -> AppResult<Html<String>> {
    let template = RegisterTemplate;
    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Forgot password page handler
async fn forgot_password_page() -> AppResult<Html<String>> {
    let template = ForgotPasswordTemplate;
    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}
