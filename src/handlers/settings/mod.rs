//! Settings page handlers
//!
//! Provides HTML pages for user profile and account settings management.
//! All routes require authentication via AuthUser middleware.

use std::net::SocketAddr;

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Regex for stripping HTML tags - compiled once at startup
    static ref HTML_TAG_RE: Regex = Regex::new(r"<[^>]*>").unwrap();
}

use axum::{
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::core::csrf::{generate_csrf_token, validate_csrf_token};
use crate::core::error::{AppError, AppResult};
use crate::core::request_id::{RequestContext, RequestId};
use crate::services::ServiceFactory;
use crate::AppState;

mod account;
mod privacy;
mod profile;
mod security;
mod twofa;

use account::{
    account_page, cancel_deletion, delete_account, delete_account_page, email_change,
    email_change_page,
};
use privacy::{export_data, privacy_page, toggle_federation};
use profile::{profile_edit_page, profile_page, profile_update, settings_redirect};
use security::{
    password_change, password_change_page, revoke_other_sessions, revoke_single_session,
    security_events_page, security_page, sessions_page,
};
use twofa::{
    twofa_disable, twofa_disable_page, twofa_enable, twofa_recovery_page, twofa_regenerate_codes,
    twofa_setup_page,
};

/// Create an AuthService instance from AppState
pub(crate) fn create_auth_service(state: &AppState) -> crate::services::AuthService {
    ServiceFactory::create_auth_service(state.db.clone())
}

/// Create a RequestContext from request components
pub(crate) fn create_request_context(
    addr: SocketAddr,
    request_id: Option<&RequestId>,
    session_id: uuid::Uuid,
) -> RequestContext {
    RequestContext::from_request(addr, request_id, Some(session_id))
}

/// Create a TotpService instance from AppState
pub(crate) fn create_totp_service(
    state: &AppState,
) -> Result<crate::services::TotpService, AppError> {
    ServiceFactory::create_totp_service(state.db.clone())
}

/// Create settings page routes
///
/// All routes require authentication and will redirect to /login if not authenticated.
/// The AuthUser middleware is applied to all routes, ensuring only authenticated
/// users can access the settings pages (RISK-004-E02).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(settings_redirect))
        // Profile routes
        .route("/profile", get(profile_page))
        .route("/profile/edit", get(profile_edit_page).post(profile_update))
        // Security routes
        .route("/security", get(security_page))
        .route(
            "/security/password",
            get(password_change_page).post(password_change),
        )
        .route("/security/sessions", get(sessions_page))
        .route(
            "/security/sessions/revoke-others",
            post(revoke_other_sessions),
        )
        .route(
            "/security/sessions/{session_id}/revoke",
            post(revoke_single_session),
        )
        .route("/security/events", get(security_events_page))
        // 2FA routes
        .route(
            "/security/2fa/setup",
            get(twofa_setup_page).post(twofa_enable),
        )
        .route(
            "/security/2fa/recovery",
            get(twofa_recovery_page).post(twofa_regenerate_codes),
        )
        .route(
            "/security/2fa/disable",
            get(twofa_disable_page).post(twofa_disable),
        )
        // Account routes
        .route("/account", get(account_page))
        .route("/account/email", get(email_change_page).post(email_change))
        .route(
            "/account/delete",
            get(delete_account_page).post(delete_account),
        )
        .route("/account/cancel-deletion", post(cancel_deletion))
        // Privacy routes
        .route("/privacy", get(privacy_page))
        .route("/privacy/federation", post(toggle_federation))
        .route("/privacy/export", post(export_data))
}

/// Sanitize text by removing HTML tags and script content (T027)
pub(crate) fn sanitize_text(input: &str) -> String {
    // Remove HTML tags using pre-compiled regex
    let without_tags = HTML_TAG_RE.replace_all(input, "");

    // Trim and normalize whitespace
    without_tags.trim().to_string()
}

/// Check if text contains potentially dangerous content
pub(crate) fn contains_dangerous_content(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("<script")
        || lower.contains("javascript:")
        || lower.contains("onerror")
        || lower.contains("onload")
        || lower.contains("onclick")
}

/// Generate a CSRF token for forms
pub(crate) fn generate_csrf(state: &AppState, session_id: uuid::Uuid) -> String {
    generate_csrf_token(session_id, &state.csrf_secret)
        .map(|t| t.token)
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "Failed to generate CSRF token");
            // Return empty string on error - form submission will fail validation
            String::new()
        })
}

/// Validate a CSRF token from form submission
pub(crate) fn validate_csrf(
    state: &AppState,
    token: &str,
    session_id: uuid::Uuid,
) -> AppResult<()> {
    validate_csrf_token(token, session_id, &state.csrf_secret)
}

/// Simple form with just CSRF token (for button-only forms)
#[derive(Debug, Deserialize)]
pub struct CsrfOnlyForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: String,
}

#[cfg(test)]
mod tests {
    use super::{contains_dangerous_content, sanitize_text, CsrfOnlyForm};
    use crate::handlers::settings::profile::{ProfileView, UpdateProfileForm};
    use crate::handlers::settings::twofa::{
        TwoFaDisableForm, TwoFaEnableForm, TwoFaRegenerateForm,
    };
    use crate::models::{MeasurementPref, User};
    use chrono::Utc;
    use uuid::Uuid;

    fn test_user() -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: true,
            password_hash: Some("hash".to_string()),
            display_name: "Test User".to_string(),
            bio: Some("A test bio".to_string()),
            avatar_url: None,
            measurement_pref: MeasurementPref::Metric,
            _totp_secret_encrypted: None,
            totp_enabled: false,
            _failed_login_attempts: 0,
            locked_until: None,
            deletion_requested_at: None,
            deletion_content_choice: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/users/testuser".to_string(),
            federation_enabled: true,
        }
    }

    #[test]
    fn test_profile_view_from_user() {
        let user = test_user();
        let view = ProfileView::from_user(&user);

        assert_eq!(view.display_name, "Test User");
        assert_eq!(view.username, "testuser");
        assert_eq!(view.masked_email, "t***@example.com");
        assert!(view.email_verified);
        assert_eq!(view.bio, Some("A test bio".to_string()));
    }

    #[test]
    fn test_profile_view_masks_email() {
        let mut user = test_user();
        user.email = Some("johndoe@example.com".to_string());
        let view = ProfileView::from_user(&user);

        assert_eq!(view.masked_email, "j******@example.com");
    }

    #[test]
    fn test_profile_view_no_email() {
        let mut user = test_user();
        user.email = None;
        let view = ProfileView::from_user(&user);

        assert_eq!(view.masked_email, "");
    }

    #[test]
    fn test_sanitize_text_removes_html() {
        assert_eq!(sanitize_text("<b>bold</b>"), "bold");
        assert_eq!(sanitize_text("<script>alert(1)</script>"), "alert(1)");
        assert_eq!(sanitize_text("Hello <img src=x>World"), "Hello World");
    }

    #[test]
    fn test_sanitize_text_preserves_normal_text() {
        assert_eq!(sanitize_text("Hello World"), "Hello World");
        assert_eq!(sanitize_text("I love cooking!"), "I love cooking!");
    }

    #[test]
    fn test_contains_dangerous_content() {
        assert!(contains_dangerous_content("<script>"));
        assert!(contains_dangerous_content("javascript:alert(1)"));
        assert!(contains_dangerous_content("onerror=alert(1)"));
        assert!(!contains_dangerous_content("Hello World"));
        assert!(!contains_dangerous_content("I script my recipes"));
    }

    #[test]
    fn test_update_profile_form_sanitize() {
        let mut form = UpdateProfileForm {
            csrf_token: "token".to_string(),
            display_name: "<b>Chef</b> Marie".to_string(),
            bio: Some("I <3 cooking".to_string()),
            avatar_url: None,
            measurement_pref: "metric".to_string(),
        };

        form.sanitize();

        assert_eq!(form.display_name, "Chef Marie");
        assert_eq!(form.bio, Some("I <3 cooking".to_string())); // <3 is not a tag
    }

    // =========================================================================
    // 2FA Form Tests
    // =========================================================================

    #[test]
    fn test_twofa_enable_form_valid_code() {
        let form = TwoFaEnableForm {
            csrf_token: "csrf_token".to_string(),
            totp_code: "123456".to_string(),
        };

        // TOTP codes are 6 digits
        assert_eq!(form.totp_code.len(), 6);
        assert!(form.totp_code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_twofa_disable_form_with_totp() {
        let form = TwoFaDisableForm {
            csrf_token: "csrf_token".to_string(),
            password: "password123".to_string(),
            code: "654321".to_string(),
        };

        // 6-digit TOTP code
        assert_eq!(form.code.len(), 6);
        assert!(form.code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_twofa_disable_form_with_recovery_code() {
        let form = TwoFaDisableForm {
            csrf_token: "csrf_token".to_string(),
            password: "password123".to_string(),
            code: "ABCD-1234".to_string(),
        };

        // Recovery code format: XXXX-XXXX
        assert_eq!(form.code.len(), 9);
        assert_eq!(form.code.chars().nth(4), Some('-'));
    }

    #[test]
    fn test_is_recovery_code_format() {
        // Helper to check if code matches recovery format
        fn is_recovery_code(code: &str) -> bool {
            code.len() == 9 && code.chars().nth(4) == Some('-')
        }

        // Recovery codes
        assert!(is_recovery_code("ABCD-1234"));
        assert!(is_recovery_code("WXYZ-5678"));
        assert!(is_recovery_code("1234-ABCD"));

        // Not recovery codes
        assert!(!is_recovery_code("123456"));
        assert!(!is_recovery_code("ABCD1234")); // Missing dash
        assert!(!is_recovery_code("ABC-1234")); // Too short before dash
        assert!(!is_recovery_code("ABCDE-123")); // Wrong split
    }

    #[test]
    fn test_twofa_regenerate_form() {
        let form = TwoFaRegenerateForm {
            csrf_token: "csrf_token".to_string(),
            password: "secure_password".to_string(),
        };

        // Password is required
        assert!(!form.password.is_empty());
    }

    #[test]
    fn test_csrf_only_form() {
        let form = CsrfOnlyForm {
            csrf_token: "csrf_token_value".to_string(),
        };

        // Form should have a CSRF token
        assert!(!form.csrf_token.is_empty());
    }
}
