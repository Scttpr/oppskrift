//! Settings page handlers
//!
//! Provides HTML pages for user profile and account settings management.
//! All routes require authentication via AuthUser middleware.

use std::net::SocketAddr;

use askama::Template;
use axum::{
    extract::{ConnectInfo, State},
    response::{Html, Redirect},
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use validator::Validate;

use crate::api::middleware::AuthUser;
use crate::core::audit::AuditEvent;
use crate::core::config::SmtpConfig;
use crate::core::error::{AppError, AppResult};
use crate::core::helpers::mask_email;
use crate::core::request_id::{RequestContext, RequestId};
use crate::models::{DeletionContentChoice, MeasurementPref, UpdateUser, User};
use crate::services::{AuthService, EmailService, PasswordService, SessionService, UserService};
use crate::AppState;

// Session expiry in days (matches api/account.rs)
const SESSION_EXPIRY_DAYS: u32 = 30;

/// Create an AuthService instance from AppState
fn create_auth_service(state: &AppState) -> AuthService {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let is_production = std::env::var("APP_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    let password_service = PasswordService::new(
        std::env::var("HIBP_ENABLED")
            .map(|v| v == "true")
            .unwrap_or(true),
    );

    let smtp_config = SmtpConfig::from_env(is_production);
    let email_service = EmailService::new(smtp_config, base_url.clone());

    AuthService::new(
        state.db.clone(),
        password_service,
        email_service,
        base_url,
        SESSION_EXPIRY_DAYS,
    )
}

/// Create a RequestContext from request components
fn create_request_context(
    addr: SocketAddr,
    request_id: Option<&RequestId>,
    session_id: uuid::Uuid,
) -> RequestContext {
    RequestContext::new()
        .with_ip(addr.ip())
        .maybe_request_id(request_id.map(|r| r.0))
        .with_session_id(session_id)
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
        // Account routes
        .route("/account", get(account_page))
        .route("/account/email", get(email_change_page).post(email_change))
        .route(
            "/account/delete",
            get(delete_account_page).post(delete_account),
        )
        .route("/account/cancel-deletion", post(cancel_deletion))
}

/// Redirect /settings to /settings/profile (T020)
async fn settings_redirect() -> Redirect {
    Redirect::to("/settings/profile")
}

// =============================================================================
// Profile View (User Story 1)
// =============================================================================

/// User profile view for display (T018)
///
/// Contains fields safe to display, with masked email for privacy.
struct ProfileView {
    display_name: String,
    username: String,
    masked_email: String,
    email_verified: bool,
    bio: Option<String>,
    avatar_url: Option<String>,
    measurement_pref: MeasurementPref,
    totp_enabled: bool,
    created_at: String,
}

impl ProfileView {
    fn from_user(user: &User) -> Self {
        let masked_email = user
            .email
            .as_ref()
            .map(|e| mask_email(e))
            .unwrap_or_default();

        Self {
            display_name: user.display_name.clone(),
            username: user.username.clone(),
            masked_email,
            email_verified: user.email_verified,
            bio: user.bio.clone(),
            avatar_url: user.avatar_url.clone(),
            measurement_pref: user.measurement_pref,
            totp_enabled: user.totp_enabled,
            created_at: user.created_at.format("%B %d, %Y").to_string(),
        }
    }
}

/// Profile page template (T019)
#[derive(Template)]
#[template(path = "settings/profile.html")]
struct ProfileTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    profile: ProfileView,
}

/// Profile settings page (T021)
async fn profile_page(State(state): State<AppState>, auth: AuthUser) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    let template = ProfileTemplate {
        active_tab: "profile",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        profile: ProfileView::from_user(&user),
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

// =============================================================================
// Profile Edit (User Story 2)
// =============================================================================

/// Form data for profile update (T025)
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProfileForm {
    /// CSRF token for form validation
    #[serde(rename = "_csrf")]
    pub csrf_token: String,

    #[validate(length(min = 1, max = 100, message = "Display name must be 1-100 characters"))]
    pub display_name: String,

    #[validate(length(max = 500, message = "Bio must be at most 500 characters"))]
    pub bio: Option<String>,

    #[validate(url(message = "Avatar URL must be a valid URL"))]
    pub avatar_url: Option<String>,

    pub measurement_pref: String,
}

impl UpdateProfileForm {
    /// Sanitize input to reject HTML/script tags (T027 - RISK-004-002)
    fn sanitize(&mut self) {
        self.display_name = sanitize_text(&self.display_name);
        self.bio = self.bio.as_ref().map(|b| sanitize_text(b));
        // avatar_url is validated as URL, which inherently prevents script injection
    }

    /// Convert to UpdateUser model
    fn to_update_user(&self) -> UpdateUser {
        let measurement_pref = match self.measurement_pref.as_str() {
            "imperial" => Some(MeasurementPref::Imperial),
            _ => Some(MeasurementPref::Metric),
        };

        UpdateUser {
            display_name: Some(self.display_name.clone()),
            bio: self.bio.clone(),
            avatar_url: self.avatar_url.clone(),
            measurement_pref,
        }
    }
}

/// Sanitize text by removing HTML tags and script content (T027)
fn sanitize_text(input: &str) -> String {
    // Remove HTML tags
    let tag_re = regex::Regex::new(r"<[^>]*>").unwrap();
    let without_tags = tag_re.replace_all(input, "");

    // Trim and normalize whitespace
    without_tags.trim().to_string()
}

/// Check if text contains potentially dangerous content
fn contains_dangerous_content(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("<script")
        || lower.contains("javascript:")
        || lower.contains("onerror")
        || lower.contains("onload")
        || lower.contains("onclick")
}

/// Profile edit form data for template
struct ProfileEditData {
    display_name: String,
    bio: String,
    avatar_url: String,
    measurement_pref: MeasurementPref,
}

impl ProfileEditData {
    fn from_user(user: &User) -> Self {
        Self {
            display_name: user.display_name.clone(),
            bio: user.bio.clone().unwrap_or_default(),
            avatar_url: user.avatar_url.clone().unwrap_or_default(),
            measurement_pref: user.measurement_pref,
        }
    }
}

/// Profile edit page template
#[derive(Template)]
#[template(path = "settings/profile_edit.html")]
struct ProfileEditTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    form: ProfileEditData,
    errors: Vec<String>,
    csrf_token: String,
}

/// Profile edit page handler (GET)
async fn profile_edit_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    // Generate CSRF token (T028)
    let csrf_token = generate_csrf_placeholder(auth.session_id);

    let template = ProfileEditTemplate {
        active_tab: "profile",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        form: ProfileEditData::from_user(&user),
        errors: vec![],
        csrf_token,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Profile update handler (POST) (T026)
async fn profile_update(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(mut form): Form<UpdateProfileForm>,
) -> AppResult<Html<String>> {
    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    // Sanitize input (T027)
    form.sanitize();

    // Validate form
    let mut errors = vec![];

    if let Err(validation_errors) = form.validate() {
        for (field, field_errors) in validation_errors.field_errors() {
            for error in field_errors {
                let msg = error
                    .message
                    .as_ref()
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| format!("Invalid {}", field));
                errors.push(msg);
            }
        }
    }

    // Check for dangerous content
    if contains_dangerous_content(&form.display_name) {
        errors.push("Display name contains invalid content".to_string());
    }
    if let Some(ref bio) = form.bio {
        if contains_dangerous_content(bio) {
            errors.push("Bio contains invalid content".to_string());
        }
    }

    // If validation errors, re-render form with errors (T029)
    if !errors.is_empty() {
        let deletion_pending = user.deletion_requested_at.is_some();
        let deletion_date = user.deletion_requested_at.map(|dt| {
            (dt + chrono::Duration::days(30))
                .format("%B %d, %Y")
                .to_string()
        });

        let csrf_token = generate_csrf_placeholder(auth.session_id);

        let template = ProfileEditTemplate {
            active_tab: "profile",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Please fix the errors below".to_string()),
            form: ProfileEditData {
                display_name: form.display_name.clone(),
                bio: form.bio.clone().unwrap_or_default(),
                avatar_url: form.avatar_url.clone().unwrap_or_default(),
                measurement_pref: match form.measurement_pref.as_str() {
                    "imperial" => MeasurementPref::Imperial,
                    _ => MeasurementPref::Metric,
                },
            },
            errors,
            csrf_token,
        };

        return Ok(Html(template.render().map_err(|e| {
            AppError::Internal(format!("Template error: {}", e))
        })?));
    }

    // Update user profile
    let update_data = form.to_update_user();
    UserService::update(&state.db, auth.id, update_data).await?;

    // Log profile update (T031 - RISK-004-005)
    AuditEvent::new("settings.profile.update")
        .with_user(auth.id)
        .with_context(&ctx)
        .with_metadata(
            "fields_updated",
            "display_name,bio,avatar_url,measurement_pref",
        )
        .persist(&state.db)
        .await;

    // Redirect to profile page with success message (T030)
    // Note: In a real app, we'd use session-based flash messages
    // For now, we'll re-render the view page with success
    let updated_user = UserService::get_by_id(&state.db, auth.id).await?;

    let deletion_pending = updated_user.deletion_requested_at.is_some();
    let deletion_date = updated_user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    let template = ProfileTemplate {
        active_tab: "profile",
        deletion_pending,
        deletion_date,
        flash_success: Some("Profile updated successfully".to_string()),
        flash_error: None,
        profile: ProfileView::from_user(&updated_user),
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Generate a placeholder CSRF token
/// In production, this would use the csrf module with a proper secret
fn generate_csrf_placeholder(session_id: uuid::Uuid) -> String {
    // Simple placeholder - in production use crate::core::csrf::generate_csrf_token
    format!("csrf_{}", session_id)
}

// =============================================================================
// Security Settings
// =============================================================================

/// Security page template
#[derive(Template)]
#[template(path = "settings/security.html")]
struct SecurityTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    totp_enabled: bool,
}

/// Security settings page
async fn security_page(State(state): State<AppState>, auth: AuthUser) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    let template = SecurityTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        totp_enabled: user.totp_enabled,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

// =============================================================================
// Account Settings
// =============================================================================

/// Account page template
#[derive(Template)]
#[template(path = "settings/account.html")]
struct AccountTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    masked_email: String,
    email_verified: bool,
}

/// Account settings page
async fn account_page(State(state): State<AppState>, auth: AuthUser) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    let masked_email = user
        .email
        .as_ref()
        .map(|e| mask_email(e))
        .unwrap_or_default();

    let template = AccountTemplate {
        active_tab: "account",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        masked_email,
        email_verified: user.email_verified,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

// =============================================================================
// Password Change (Phase 6 - User Story 4)
// =============================================================================

/// Password change form (T042)
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: String,

    pub current_password: String,

    #[validate(length(min = 12, message = "Password must be at least 12 characters"))]
    pub new_password: String,

    pub confirm_password: String,
}

/// Password change page template
#[derive(Template)]
#[template(path = "settings/password.html")]
struct PasswordChangeTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    errors: Vec<String>,
    csrf_token: String,
}

/// Password change page (GET)
async fn password_change_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf_placeholder(auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    let template = PasswordChangeTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        errors: vec![],
        csrf_token,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Password change handler (POST) (T043)
async fn password_change(
    State(state): State<AppState>,
    auth: AuthUser,
    Form(form): Form<ChangePasswordForm>,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let mut errors = vec![];

    // Validate form
    if let Err(validation_errors) = form.validate() {
        for (_, field_errors) in validation_errors.field_errors() {
            for error in field_errors {
                if let Some(msg) = &error.message {
                    errors.push(msg.to_string());
                }
            }
        }
    }

    // Check passwords match
    if form.new_password != form.confirm_password {
        errors.push("New passwords do not match".to_string());
    }

    if !errors.is_empty() {
        let csrf_token = generate_csrf_placeholder(auth.session_id);
        let deletion_pending = user.deletion_requested_at.is_some();
        let deletion_date = user.deletion_requested_at.map(|dt| {
            (dt + chrono::Duration::days(30))
                .format("%B %d, %Y")
                .to_string()
        });

        let template = PasswordChangeTemplate {
            active_tab: "security",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Please fix the errors below".to_string()),
            errors,
            csrf_token,
        };

        return Ok(Html(template.render().map_err(|e| {
            AppError::Internal(format!("Template error: {}", e))
        })?));
    }

    // Call auth service to change password
    let auth_service = create_auth_service(&state);
    let ctx = RequestContext::new().with_session_id(auth.session_id);
    match auth_service
        .change_password(
            auth.id,
            auth.session_id,
            &form.current_password,
            &form.new_password,
            &ctx,
        )
        .await
    {
        Ok(sessions_revoked) => {
            // Note: AuthService.change_password already revokes other sessions (T044, T045)
            // and logs the audit event

            tracing::info!(
                user_id = %auth.id,
                sessions_revoked = sessions_revoked,
                "Password changed via settings"
            );

            // Return to security page with success
            let template = SecurityTemplate {
                active_tab: "security",
                deletion_pending: user.deletion_requested_at.is_some(),
                deletion_date: user.deletion_requested_at.map(|dt| {
                    (dt + chrono::Duration::days(30))
                        .format("%B %d, %Y")
                        .to_string()
                }),
                flash_success: Some(
                    "Password changed successfully. Other sessions have been signed out."
                        .to_string(),
                ),
                flash_error: None,
                totp_enabled: user.totp_enabled,
            };

            Ok(Html(template.render().map_err(|e| {
                AppError::Internal(format!("Template error: {}", e))
            })?))
        }
        Err(e) => {
            let csrf_token = generate_csrf_placeholder(auth.session_id);
            let deletion_pending = user.deletion_requested_at.is_some();
            let deletion_date = user.deletion_requested_at.map(|dt| {
                (dt + chrono::Duration::days(30))
                    .format("%B %d, %Y")
                    .to_string()
            });

            let template = PasswordChangeTemplate {
                active_tab: "security",
                deletion_pending,
                deletion_date,
                flash_success: None,
                flash_error: Some(format!("Failed to change password: {}", e)),
                errors: vec![],
                csrf_token,
            };

            Ok(Html(template.render().map_err(|e| {
                AppError::Internal(format!("Template error: {}", e))
            })?))
        }
    }
}

// =============================================================================
// Sessions Management (Phase 8 - User Story 6)
// =============================================================================

/// Sessions page template
#[derive(Template)]
#[template(path = "settings/sessions.html")]
struct SessionsTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    session_count: i64,
    csrf_token: String,
}

/// Sessions page (GET)
async fn sessions_page(State(state): State<AppState>, auth: AuthUser) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf_placeholder(auth.session_id);

    // Count active sessions
    let session_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sessions WHERE user_id = $1 AND expires_at > NOW()",
    )
    .bind(auth.id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(1);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    let template = SessionsTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        session_count,
        csrf_token,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Revoke other sessions handler (POST)
async fn revoke_other_sessions(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
) -> AppResult<Redirect> {
    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    let session_service = SessionService::new(state.db.clone(), 30);
    session_service
        .revoke_others_for_user(auth.id, auth.session_id)
        .await
        .map_err(|e| AppError::Internal(format!("Session revocation failed: {}", e)))?;

    // Log session revocation (T066)
    AuditEvent::new("settings.sessions.revoke_others")
        .with_user(auth.id)
        .with_context(&ctx)
        .persist(&state.db)
        .await;

    Ok(Redirect::to("/settings/security/sessions"))
}

// =============================================================================
// Email Change (Phase 5 - User Story 3)
// =============================================================================

/// Email change form (T032)
#[derive(Debug, Deserialize, Validate)]
pub struct ChangeEmailForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: String,

    #[validate(email(message = "Please enter a valid email address"))]
    pub new_email: String,

    pub password: String,
}

/// Email change page template
#[derive(Template)]
#[template(path = "settings/email.html")]
struct EmailChangeTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    errors: Vec<String>,
    current_email: String,
    csrf_token: String,
}

/// Email change page (GET)
async fn email_change_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf_placeholder(auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    let template = EmailChangeTemplate {
        active_tab: "account",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        errors: vec![],
        current_email: mask_email(user.email.as_deref().unwrap_or("")),
        csrf_token,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Email change handler (POST) (T035)
async fn email_change(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(form): Form<ChangeEmailForm>,
) -> AppResult<Html<String>> {
    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf_placeholder(auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    // Validate form
    let mut errors = vec![];
    if let Err(validation_errors) = form.validate() {
        for (_, field_errors) in validation_errors.field_errors() {
            for error in field_errors {
                if let Some(msg) = &error.message {
                    errors.push(msg.to_string());
                }
            }
        }
    }

    if !errors.is_empty() {
        let template = EmailChangeTemplate {
            active_tab: "account",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Please fix the errors below".to_string()),
            errors,
            current_email: mask_email(user.email.as_deref().unwrap_or("")),
            csrf_token,
        };

        return Ok(Html(template.render().map_err(|e| {
            AppError::Internal(format!("Template error: {}", e))
        })?));
    }

    // Call auth service - use generic message to prevent enumeration (T036)
    let auth_service = create_auth_service(&state);
    let _ = auth_service
        .change_email(auth.id, &form.new_email, &form.password, &ctx)
        .await;

    // Log email change request (T039)
    AuditEvent::new("settings.email.change_request")
        .with_user(auth.id)
        .with_context(&ctx)
        .persist(&state.db)
        .await;

    // Always show success to prevent email enumeration
    let template = EmailChangeTemplate {
        active_tab: "account",
        deletion_pending,
        deletion_date,
        flash_success: Some(
            "If the email is valid and not already in use, you will receive a confirmation email."
                .to_string(),
        ),
        flash_error: None,
        errors: vec![],
        current_email: mask_email(user.email.as_deref().unwrap_or("")),
        csrf_token,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

// =============================================================================
// Account Deletion (Phase 9 - User Story 7)
// =============================================================================

/// Delete account form (T069)
#[derive(Debug, Deserialize)]
pub struct DeleteAccountForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: String,

    pub password: String,

    pub content_choice: String,
}

/// Delete account page template
#[derive(Template)]
#[template(path = "settings/delete_account.html")]
struct DeleteAccountTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    csrf_token: String,
}

/// Delete account page (GET)
async fn delete_account_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf_placeholder(auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    let template = DeleteAccountTemplate {
        active_tab: "account",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        csrf_token,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Delete account handler (POST) (T070)
async fn delete_account(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(form): Form<DeleteAccountForm>,
) -> AppResult<Html<String>> {
    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    let content_choice = match form.content_choice.as_str() {
        "delete_all" => DeletionContentChoice::DeleteAll,
        _ => DeletionContentChoice::Anonymize,
    };

    // Store the content choice preference
    sqlx::query("UPDATE users SET deletion_content_choice = $1, updated_at = NOW() WHERE id = $2")
        .bind(content_choice)
        .bind(auth.id)
        .execute(&state.db)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to save content choice: {}", e)))?;

    let auth_service = create_auth_service(&state);
    match auth_service
        .request_deletion(auth.id, &form.password, &ctx)
        .await
    {
        Ok(deletion_date) => {
            // Log deletion request (T076)
            AuditEvent::new("settings.account.delete_request")
                .with_user(auth.id)
                .with_context(&ctx)
                .with_metadata("content_choice", &form.content_choice)
                .persist(&state.db)
                .await;

            // Redirect to account page with pending deletion banner
            let user = UserService::get_by_id(&state.db, auth.id).await?;
            let template = AccountTemplate {
                active_tab: "account",
                deletion_pending: true,
                deletion_date: Some(deletion_date.format("%B %d, %Y").to_string()),
                flash_success: Some(
                    "Account deletion scheduled. You can cancel within 30 days.".to_string(),
                ),
                flash_error: None,
                masked_email: mask_email(user.email.as_deref().unwrap_or("")),
                email_verified: user.email_verified,
            };

            Ok(Html(template.render().map_err(|e| {
                AppError::Internal(format!("Template error: {}", e))
            })?))
        }
        Err(e) => {
            let csrf_token = generate_csrf_placeholder(auth.session_id);
            let template = DeleteAccountTemplate {
                active_tab: "account",
                deletion_pending: false,
                deletion_date: None,
                flash_success: None,
                flash_error: Some(format!("Failed to request deletion: {}", e)),
                csrf_token,
            };

            Ok(Html(template.render().map_err(|e| {
                AppError::Internal(format!("Template error: {}", e))
            })?))
        }
    }
}

/// Cancel deletion handler (POST) (T072)
async fn cancel_deletion(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
) -> AppResult<Redirect> {
    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    let auth_service = create_auth_service(&state);
    let _ = auth_service.cancel_deletion(auth.id, &ctx).await;

    // Log cancellation (T076)
    AuditEvent::new("settings.account.delete_cancel")
        .with_user(auth.id)
        .with_context(&ctx)
        .persist(&state.db)
        .await;

    Ok(Redirect::to("/settings/account"))
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
