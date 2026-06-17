use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
    routing::get,
    Router,
};

use crate::core::error::AppResult;
use crate::services::ServiceFactory;
use crate::AppState;

/// Create an AuthService instance from AppState
fn create_auth_service(state: &AppState) -> crate::services::AuthService {
    ServiceFactory::create_auth_service(state.db.clone())
}

/// Auth page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_page))
        .route("/register", get(register_page))
        .route("/forgot-password", get(forgot_password_page))
        .route("/reset-password", get(reset_password_page))
        .route("/confirm-email/{token}", get(confirm_email_page))
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

/// Reset password page template
#[derive(Template)]
#[template(path = "auth/reset_password.html")]
struct ResetPasswordTemplate;

/// Email confirmed page template
#[derive(Template)]
#[template(path = "auth/email_confirmed.html")]
struct EmailConfirmedTemplate {
    success: bool,
    error_message: String,
}

/// Login page handler
async fn login_page() -> AppResult<Html<String>> {
    let template = LoginTemplate;
    crate::core::render(&template)
}

/// Register page handler
async fn register_page() -> AppResult<Html<String>> {
    let template = RegisterTemplate;
    crate::core::render(&template)
}

/// Forgot password page handler
async fn forgot_password_page() -> AppResult<Html<String>> {
    let template = ForgotPasswordTemplate;
    crate::core::render(&template)
}

/// Reset password page handler
async fn reset_password_page() -> AppResult<Html<String>> {
    let template = ResetPasswordTemplate;
    crate::core::render(&template)
}

/// Email confirmation page handler
///
/// Confirms the email using the token and displays success/error page.
async fn confirm_email_page(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<Html<String>> {
    // Create auth service and attempt confirmation
    let auth_service = create_auth_service(&state);

    let (success, error_message) = match auth_service
        .confirm_email(&token, &crate::core::RequestContext::default())
        .await
    {
        Ok(_) => (true, String::new()),
        Err(e) => {
            let msg = match e {
                crate::services::AuthError::InvalidToken => {
                    "This confirmation link is invalid or has expired. Please request a new one."
                        .to_string()
                }
                crate::services::AuthError::AlreadyVerified => {
                    "Your email has already been verified. You can log in now.".to_string()
                }
                crate::services::AuthError::UserNotFound => {
                    "This confirmation link is invalid. Please register again.".to_string()
                }
                _ => "An error occurred while confirming your email. Please try again.".to_string(),
            };
            (false, msg)
        }
    };

    let template = EmailConfirmedTemplate {
        success,
        error_message,
    };

    crate::core::render(&template)
}

#[cfg(test)]
mod tests {
    use super::*;
    use askama::Template;

    // ==========================================================================
    // Route Configuration Tests (T051)
    // ==========================================================================

    #[test]
    fn test_routes_returns_router() {
        let router = routes();
        // Router created successfully - this validates the route configuration
        let _ = router;
    }

    // ==========================================================================
    // Template Rendering Tests (T051)
    // ==========================================================================

    #[test]
    fn test_login_template_renders() {
        let template = LoginTemplate;
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("login") || html.contains("Login") || html.contains("form"));
    }

    #[test]
    fn test_register_template_renders() {
        let template = RegisterTemplate;
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("register") || html.contains("Register") || html.contains("form"));
    }

    #[test]
    fn test_forgot_password_template_renders() {
        let template = ForgotPasswordTemplate;
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("password") || html.contains("Password") || html.contains("email"));
    }

    #[test]
    fn test_reset_password_template_renders() {
        let template = ResetPasswordTemplate;
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("password") || html.contains("Password") || html.contains("reset"));
    }

    // ==========================================================================
    // Template HTML Structure Tests (T051)
    // ==========================================================================

    #[test]
    fn test_login_template_contains_form() {
        let template = LoginTemplate;
        let html = template.render().unwrap();
        // Should contain a form element
        assert!(html.contains("<form") || html.contains("form"));
    }

    #[test]
    fn test_register_template_contains_form() {
        let template = RegisterTemplate;
        let html = template.render().unwrap();
        assert!(html.contains("<form") || html.contains("form"));
    }

    #[test]
    fn test_templates_produce_valid_html() {
        // All templates should produce non-empty HTML
        let login = LoginTemplate.render().unwrap();
        let register = RegisterTemplate.render().unwrap();
        let forgot = ForgotPasswordTemplate.render().unwrap();
        let reset = ResetPasswordTemplate.render().unwrap();

        assert!(!login.is_empty());
        assert!(!register.is_empty());
        assert!(!forgot.is_empty());
        assert!(!reset.is_empty());
    }

    // ==========================================================================
    // Handler Async Tests (T051)
    // ==========================================================================

    #[tokio::test]
    async fn test_login_page_handler() {
        let result = login_page().await;
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.0.is_empty());
    }

    #[tokio::test]
    async fn test_register_page_handler() {
        let result = register_page().await;
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.0.is_empty());
    }

    #[tokio::test]
    async fn test_forgot_password_page_handler() {
        let result = forgot_password_page().await;
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.0.is_empty());
    }

    #[tokio::test]
    async fn test_reset_password_page_handler() {
        let result = reset_password_page().await;
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.0.is_empty());
    }
}
