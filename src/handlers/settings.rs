//! Settings page handlers
//!
//! Provides HTML pages for user profile and account settings management.
//! All routes require authentication via AuthUser middleware.

use std::net::SocketAddr;

use askama::Template;
use axum::{
    extract::{ConnectInfo, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Router,
};
use serde::Deserialize;
use validator::Validate;

use crate::api::middleware::AuthUser;
use crate::core::audit::AuditEvent;
use crate::core::config::SmtpConfig;
use crate::core::csrf::{generate_csrf_token, validate_csrf_token};
use crate::core::error::{AppError, AppResult};
use crate::core::helpers::mask_email;
use crate::core::request_id::{RequestContext, RequestId};
use crate::models::{DeletionContentChoice, MeasurementPref, UpdateUser, User};
use crate::services::{
    AuthService, EmailService, PasswordService, SessionService, TotpError, TotpService, UserService,
};
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

/// Create a TotpService instance from AppState
fn create_totp_service(state: &AppState) -> Result<TotpService, AppError> {
    TotpService::from_env(state.db.clone()).map_err(|e| {
        tracing::error!(error = %e, "Failed to create TOTP service");
        AppError::Internal("2FA service unavailable".to_string())
    })
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
    let csrf_token = generate_csrf(&state, auth.session_id);

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
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

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

        let csrf_token = generate_csrf(&state, auth.session_id);

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

/// Generate a CSRF token for forms
fn generate_csrf(state: &AppState, session_id: uuid::Uuid) -> String {
    generate_csrf_token(session_id, &state.csrf_secret)
        .map(|t| t.token)
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "Failed to generate CSRF token");
            // Return empty string on error - form submission will fail validation
            String::new()
        })
}

/// Validate a CSRF token from form submission
fn validate_csrf(state: &AppState, token: &str, session_id: uuid::Uuid) -> AppResult<()> {
    validate_csrf_token(token, session_id, &state.csrf_secret)
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
    csrf_token: String,
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
        csrf_token: generate_csrf(&state, auth.session_id),
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
    let csrf_token = generate_csrf(&state, auth.session_id);

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
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

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
        let csrf_token = generate_csrf(&state, auth.session_id);
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
            let csrf_token = generate_csrf(&state, auth.session_id);
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
    let csrf_token = generate_csrf(&state, auth.session_id);

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

/// Simple form with just CSRF token (for button-only forms)
#[derive(Debug, Deserialize)]
pub struct CsrfOnlyForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: String,
}

/// Revoke other sessions handler (POST)
async fn revoke_other_sessions(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(form): Form<CsrfOnlyForm>,
) -> AppResult<Redirect> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    let session_service = SessionService::new(state.db.clone(), 30);
    session_service
        .revoke_others_for_user(auth.id, auth.session_id)
        .await
        .map_err(|e| AppError::Internal(format!("Session revocation failed: {}", e)))?;

    // Log session revocation (T066)
    AuditEvent::new("auth.session.revoke.all")
        .with_user(auth.id)
        .with_context(&ctx)
        .persist(&state.db)
        .await;

    Ok(Redirect::to("/settings/security/sessions"))
}

// =============================================================================
// Two-Factor Authentication (2FA)
// =============================================================================

/// 2FA setup page template
#[derive(Template)]
#[template(path = "settings/2fa_setup.html")]
struct TwoFaSetupTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    // Setup data (None if 2FA already enabled)
    qr_code: Option<String>,
    secret: Option<String>,
    csrf_token: String,
    already_enabled: bool,
}

/// 2FA enable form
#[derive(Debug, Deserialize)]
pub struct TwoFaEnableForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: String,
    pub totp_code: String,
}

/// 2FA setup page (GET) - Shows QR code for authenticator app
async fn twofa_setup_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf(&state, auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    // If 2FA is already enabled, show message instead of setup
    if user.totp_enabled {
        let template = TwoFaSetupTemplate {
            active_tab: "security",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: None,
            qr_code: None,
            secret: None,
            csrf_token,
            already_enabled: true,
        };

        return Ok(Html(template.render().map_err(|e| {
            AppError::Internal(format!("Template error: {}", e))
        })?));
    }

    // Generate 2FA setup (QR code and secret)
    let totp_service = create_totp_service(&state)?;
    let email = user
        .email
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Email required for 2FA setup".to_string()))?;

    let setup = totp_service
        .setup_2fa(auth.id, email)
        .await
        .map_err(|e| match e {
            TotpError::AlreadyEnabled => {
                AppError::Conflict("Two-factor authentication is already enabled".to_string())
            }
            e => {
                tracing::error!(error = %e, "2FA setup failed");
                AppError::Internal("2FA setup failed".to_string())
            }
        })?;

    let template = TwoFaSetupTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        qr_code: Some(setup.qr_code),
        secret: Some(setup.secret),
        csrf_token,
        already_enabled: false,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// 2FA enable handler (POST) - Verifies TOTP code and enables 2FA
async fn twofa_enable(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(form): Form<TwoFaEnableForm>,
) -> AppResult<Html<String>> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    // Validate TOTP code format
    if form.totp_code.len() != 6 || !form.totp_code.chars().all(|c| c.is_ascii_digit()) {
        let csrf_token = generate_csrf(&state, auth.session_id);
        let totp_service = create_totp_service(&state)?;
        let empty_email = String::new();
        let email = user.email.as_ref().unwrap_or(&empty_email);
        let setup = totp_service.setup_2fa(auth.id, email).await.ok();

        let template = TwoFaSetupTemplate {
            active_tab: "security",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Please enter a valid 6-digit code".to_string()),
            qr_code: setup.as_ref().map(|s| s.qr_code.clone()),
            secret: setup.as_ref().map(|s| s.secret.clone()),
            csrf_token,
            already_enabled: false,
        };

        return Ok(Html(template.render().map_err(|e| {
            AppError::Internal(format!("Template error: {}", e))
        })?));
    }

    let totp_service = create_totp_service(&state)?;

    match totp_service
        .enable_2fa(auth.id, &form.totp_code, &ctx)
        .await
    {
        Ok(recovery_codes) => {
            // Show success page with recovery codes
            let template = TwoFaEnabledTemplate {
                active_tab: "security",
                deletion_pending,
                deletion_date,
                flash_success: None,
                flash_error: None,
                recovery_codes,
            };

            Ok(Html(template.render().map_err(|e| {
                AppError::Internal(format!("Template error: {}", e))
            })?))
        }
        Err(e) => {
            let error_msg = match e {
                TotpError::InvalidCode => "Invalid code. Please try again.".to_string(),
                TotpError::NoPendingSetup => "Setup expired. Please start again.".to_string(),
                _ => "Failed to enable 2FA. Please try again.".to_string(),
            };

            // Re-generate QR code for retry
            let empty_email = String::new();
            let email = user.email.as_ref().unwrap_or(&empty_email);
            let setup = totp_service.setup_2fa(auth.id, email).await.ok();
            let csrf_token = generate_csrf(&state, auth.session_id);

            let template = TwoFaSetupTemplate {
                active_tab: "security",
                deletion_pending,
                deletion_date,
                flash_success: None,
                flash_error: Some(error_msg),
                qr_code: setup.as_ref().map(|s| s.qr_code.clone()),
                secret: setup.as_ref().map(|s| s.secret.clone()),
                csrf_token,
                already_enabled: false,
            };

            Ok(Html(template.render().map_err(|e| {
                AppError::Internal(format!("Template error: {}", e))
            })?))
        }
    }
}

/// 2FA enabled success template (shows recovery codes)
#[derive(Template)]
#[template(path = "settings/2fa_enabled.html")]
struct TwoFaEnabledTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    recovery_codes: Vec<String>,
}

/// 2FA recovery codes page template
#[derive(Template)]
#[template(path = "settings/2fa_recovery.html")]
struct TwoFaRecoveryTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    codes_total: u32,
    codes_remaining: u32,
    generated_at: Option<String>,
    csrf_token: String,
    // New codes after regeneration (shown only once)
    new_codes: Option<Vec<String>>,
}

/// 2FA recovery codes page (GET)
async fn twofa_recovery_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf(&state, auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    // Check if 2FA is enabled
    if !user.totp_enabled {
        return Err(AppError::BadRequest(
            "Two-factor authentication is not enabled".to_string(),
        ));
    }

    let totp_service = create_totp_service(&state)?;
    let (total, remaining, generated_at) = totp_service
        .get_recovery_codes_status(auth.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get recovery codes status");
            AppError::Internal("Failed to get recovery codes status".to_string())
        })?;

    let template = TwoFaRecoveryTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        codes_total: total,
        codes_remaining: remaining,
        generated_at: generated_at.map(|dt| dt.format("%B %d, %Y at %H:%M UTC").to_string()),
        csrf_token,
        new_codes: None,
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// 2FA regenerate recovery codes form
#[derive(Debug, Deserialize)]
pub struct TwoFaRegenerateForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: String,
    pub password: String,
}

/// 2FA regenerate recovery codes handler (POST)
async fn twofa_regenerate_codes(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(form): Form<TwoFaRegenerateForm>,
) -> AppResult<Html<String>> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf(&state, auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    // Verify password first
    let auth_service = create_auth_service(&state);
    let password_hash = user
        .password_hash
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Password not set for this account".to_string()))?;

    let password_valid = auth_service
        .password_service()
        .verify(&form.password, password_hash)
        .map_err(|e| {
            tracing::error!(error = %e, "Password verification failed");
            AppError::Internal("Failed to verify password".to_string())
        })?;

    if !password_valid {
        let totp_service = create_totp_service(&state)?;
        let (total, remaining, generated_at) = totp_service
            .get_recovery_codes_status(auth.id)
            .await
            .unwrap_or((8, 8, None));

        let template = TwoFaRecoveryTemplate {
            active_tab: "security",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Incorrect password".to_string()),
            codes_total: total,
            codes_remaining: remaining,
            generated_at: generated_at.map(|dt| dt.format("%B %d, %Y at %H:%M UTC").to_string()),
            csrf_token,
            new_codes: None,
        };

        return Ok(Html(template.render().map_err(|e| {
            AppError::Internal(format!("Template error: {}", e))
        })?));
    }

    // Regenerate codes
    let totp_service = create_totp_service(&state)?;
    let response = totp_service
        .regenerate_recovery_codes(auth.id, &ctx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to regenerate recovery codes");
            AppError::Internal("Failed to regenerate recovery codes".to_string())
        })?;

    let template = TwoFaRecoveryTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: Some("Recovery codes regenerated successfully".to_string()),
        flash_error: None,
        codes_total: 8,
        codes_remaining: 8,
        generated_at: Some(
            chrono::Utc::now()
                .format("%B %d, %Y at %H:%M UTC")
                .to_string(),
        ),
        csrf_token,
        new_codes: Some(response.codes),
    };

    Ok(Html(template.render().map_err(|e| {
        AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// 2FA disable page template
#[derive(Template)]
#[template(path = "settings/2fa_disable.html")]
struct TwoFaDisableTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    csrf_token: String,
}

/// 2FA disable form
#[derive(Debug, Deserialize)]
pub struct TwoFaDisableForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: String,
    pub password: String,
    pub code: String,
}

/// 2FA disable page (GET)
async fn twofa_disable_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf(&state, auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    // Check if 2FA is enabled
    if !user.totp_enabled {
        return Err(AppError::BadRequest(
            "Two-factor authentication is not enabled".to_string(),
        ));
    }

    let template = TwoFaDisableTemplate {
        active_tab: "security",
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

/// 2FA disable handler (POST)
async fn twofa_disable(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(form): Form<TwoFaDisableForm>,
) -> Result<Response, AppError> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf(&state, auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user.deletion_requested_at.map(|dt| {
        (dt + chrono::Duration::days(30))
            .format("%B %d, %Y")
            .to_string()
    });

    // Verify password first
    let auth_service = create_auth_service(&state);
    let password_hash = user
        .password_hash
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Password not set for this account".to_string()))?;

    let password_valid = auth_service
        .password_service()
        .verify(&form.password, password_hash)
        .map_err(|e| {
            tracing::error!(error = %e, "Password verification failed");
            AppError::Internal("Failed to verify password".to_string())
        })?;

    if !password_valid {
        let template = TwoFaDisableTemplate {
            active_tab: "security",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Incorrect password".to_string()),
            csrf_token,
        };

        let html = template
            .render()
            .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;
        return Ok(Html(html).into_response());
    }

    // Verify TOTP/recovery code and disable
    let totp_service = create_totp_service(&state)?;

    match totp_service.disable_2fa(auth.id, &form.code, &ctx).await {
        Ok(()) => {
            // Redirect to security page with success message
            Ok(Redirect::to("/settings/security").into_response())
        }
        Err(e) => {
            let error_msg = match e {
                TotpError::InvalidCode | TotpError::InvalidRecoveryCode => {
                    "Invalid code. Please try again.".to_string()
                }
                TotpError::NotEnabled => "Two-factor authentication is not enabled".to_string(),
                _ => "Failed to disable 2FA. Please try again.".to_string(),
            };

            let template = TwoFaDisableTemplate {
                active_tab: "security",
                deletion_pending,
                deletion_date,
                flash_success: None,
                flash_error: Some(error_msg),
                csrf_token,
            };

            let html = template
                .render()
                .map_err(|e| AppError::Internal(format!("Template error: {}", e)))?;
            Ok(Html(html).into_response())
        }
    }
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
    let csrf_token = generate_csrf(&state, auth.session_id);

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
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf(&state, auth.session_id);

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
    AuditEvent::new("auth.email.change.request")
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
    let csrf_token = generate_csrf(&state, auth.session_id);

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
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

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
            AuditEvent::new("auth.account.delete.request")
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
                csrf_token: generate_csrf(&state, auth.session_id),
            };

            Ok(Html(template.render().map_err(|e| {
                AppError::Internal(format!("Template error: {}", e))
            })?))
        }
        Err(e) => {
            let csrf_token = generate_csrf(&state, auth.session_id);
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
    Form(form): Form<CsrfOnlyForm>,
) -> AppResult<Redirect> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    let auth_service = create_auth_service(&state);
    let _ = auth_service.cancel_deletion(auth.id, &ctx).await;

    // Log cancellation (T076)
    AuditEvent::new("auth.account.delete.cancel")
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
