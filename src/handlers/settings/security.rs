//! Security settings handlers (password, sessions, security events)

use std::net::SocketAddr;

use askama::Template;
use axum::{
    extract::{ConnectInfo, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;
use validator::Validate;

use super::{
    create_auth_service, create_request_context, generate_csrf, validate_csrf, CsrfOnlyForm,
};
use crate::api::account::SecurityEventView;
use crate::api::middleware::{AuthUser, Viewer};
use crate::core::audit::AuditEvent;
use crate::core::error::{AppError, AppResult};
use crate::core::request_id::{RequestContext, RequestId};
use crate::models::session::SessionItemView;
use crate::services::{SecurityEventService, ServiceFactory};
use crate::AppState;

// =============================================================================
// Security Settings
// =============================================================================

/// Security page template
#[derive(Template)]
#[template(path = "settings/security.html")]
pub(crate) struct SecurityTemplate {
    pub(crate) active_tab: &'static str,
    pub(crate) deletion_pending: bool,
    pub(crate) deletion_date: Option<String>,
    pub(crate) flash_success: Option<String>,
    pub(crate) flash_error: Option<String>,
    pub(crate) totp_enabled: bool,
}

/// Security settings page
pub(crate) async fn security_page(viewer: Viewer) -> AppResult<Html<String>> {
    let user = viewer.user;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    let template = SecurityTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        totp_enabled: user.totp_enabled,
    };

    crate::core::render(&template)
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

    #[validate(length(
        min = 12,
        message = "Le mot de passe doit comporter au moins 12 caractères"
    ))]
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
pub(crate) async fn password_change_page(
    State(state): State<AppState>,
    viewer: Viewer,
) -> AppResult<Html<String>> {
    let user = viewer.user;
    let csrf_token = generate_csrf(&state, viewer.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    let template = PasswordChangeTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        errors: vec![],
        csrf_token,
    };

    crate::core::render(&template)
}

/// Password change handler (POST) (T043)
pub(crate) async fn password_change(
    State(state): State<AppState>,
    viewer: Viewer,
    Form(form): Form<ChangePasswordForm>,
) -> AppResult<Html<String>> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, viewer.session_id)?;

    let user = viewer.user;
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
        let csrf_token = generate_csrf(&state, viewer.session_id);
        let deletion_pending = user.deletion_requested_at.is_some();
        let deletion_date = user
            .deletion_requested_at
            .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

        let template = PasswordChangeTemplate {
            active_tab: "security",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Corrige les erreurs ci-dessous".to_string()),
            errors,
            csrf_token,
        };

        return crate::core::render(&template);
    }

    // Call auth service to change password
    let auth_service = create_auth_service(&state);
    let ctx = RequestContext::new().with_session_id(viewer.session_id);
    match auth_service
        .change_password(
            viewer.id,
            viewer.session_id,
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
                user_id = %viewer.id,
                sessions_revoked = sessions_revoked,
                "Password changed via settings"
            );

            // Return to security page with success
            let template = SecurityTemplate {
                active_tab: "security",
                deletion_pending: user.deletion_requested_at.is_some(),
                deletion_date: user.deletion_requested_at.map(|dt| {
                    crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30)))
                }),
                flash_success: Some(
                    "Mot de passe modifié avec succès. Les autres sessions ont été déconnectées."
                        .to_string(),
                ),
                flash_error: None,
                totp_enabled: user.totp_enabled,
            };

            crate::core::render(&template)
        }
        Err(e) => {
            let csrf_token = generate_csrf(&state, viewer.session_id);
            let deletion_pending = user.deletion_requested_at.is_some();
            let deletion_date = user
                .deletion_requested_at
                .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

            let template = PasswordChangeTemplate {
                active_tab: "security",
                deletion_pending,
                deletion_date,
                flash_success: None,
                flash_error: Some(format!("Failed to change password: {}", e)),
                errors: vec![],
                csrf_token,
            };

            crate::core::render(&template)
        }
    }
}

// =============================================================================
// Sessions Management (Phase 3 - User Story 1)
// =============================================================================

/// Sessions page template (T009)
#[derive(Template)]
#[template(path = "settings/sessions.html")]
struct SessionsPageTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    sessions: Vec<SessionItemView>,
    session_count: i64,
    csrf_token: String,
}

/// Sessions page handler (GET) (T010)
pub(crate) async fn sessions_page(
    State(state): State<AppState>,
    viewer: Viewer,
) -> AppResult<Html<String>> {
    let user = viewer.user;
    let csrf_token = generate_csrf(&state, viewer.session_id);

    // Get all active sessions with details
    let session_service = ServiceFactory::create_session_service(state.db.clone());
    let session_infos = session_service
        .list_for_user(viewer.id, Some(viewer.session_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list sessions");
            AppError::Internal("Failed to load sessions".to_string())
        })?;

    // Convert to view models
    let sessions: Vec<SessionItemView> = session_infos
        .iter()
        .map(|info| {
            // Convert service::SessionInfo to models::SessionInfo first
            let model_info = crate::models::SessionInfo {
                id: info.id,
                device_info: info.device_info.clone(),
                ip_address: info.ip_address.clone(),
                last_activity: info.last_activity,
                created_at: info.created_at,
                is_current: info.is_current,
            };
            SessionItemView::from_session_info(&model_info)
        })
        .collect();

    let session_count = sessions.len() as i64;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    let template = SessionsPageTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        sessions,
        session_count,
        csrf_token,
    };

    crate::core::render(&template)
}

/// Revoke other sessions handler (POST)
pub(crate) async fn revoke_other_sessions(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(form): Form<CsrfOnlyForm>,
) -> AppResult<Redirect> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    let session_service = ServiceFactory::create_session_service(state.db.clone());
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

/// Revoke single session handler (POST) (T011)
///
/// HTMX endpoint to revoke a specific session.
/// Returns HTML fragment for the session row (removed or empty).
pub(crate) async fn revoke_single_session(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    axum::extract::Path(session_id): axum::extract::Path<uuid::Uuid>,
    Form(form): Form<CsrfOnlyForm>,
) -> Result<Response, AppError> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    // Prevent revoking current session
    if session_id == auth.session_id {
        return Err(AppError::BadRequest(
            "Cannot revoke current session. Use logout instead.".to_string(),
        ));
    }

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    // Verify session belongs to user before revoking
    let session_service = ServiceFactory::create_session_service(state.db.clone());
    let sessions = session_service
        .list_for_user(auth.id, None)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to verify session ownership");
            AppError::Internal("Failed to revoke session".to_string())
        })?;

    if !sessions.iter().any(|s| s.id == session_id) {
        return Err(AppError::NotFound("Session not found".to_string()));
    }

    // Revoke session
    session_service
        .revoke_by_id(session_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to revoke session");
            AppError::Internal("Failed to revoke session".to_string())
        })?;

    // Log security event
    AuditEvent::new("auth.session.revoke")
        .with_user(auth.id)
        .with_context(&ctx)
        .with_metadata("session_id", &session_id.to_string())
        .persist(&state.db)
        .await;

    // Return empty response for HTMX to swap out the row
    Ok(Html("").into_response())
}

// =============================================================================
// Security Events (Phase 4 - User Story 2)
// =============================================================================

/// Pagination info for security events
#[derive(Debug, Clone)]
struct PaginationInfo {
    current_page: u32,
    total_pages: u32,
    has_prev: bool,
    has_next: bool,
    prev_page: u32,
    next_page: u32,
}

/// Security events page template (T014)
#[derive(Template)]
#[template(path = "settings/security_events.html")]
struct SecurityEventsPageTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    events: Vec<SecurityEventView>,
    pagination: PaginationInfo,
}

/// Security events page handler (GET) (T015)
///
/// Displays paginated security events for the authenticated user.
/// Default pagination: 20 events per page, most recent first.
pub(crate) async fn security_events_page(
    State(state): State<AppState>,
    viewer: Viewer,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> AppResult<Html<String>> {
    let user = viewer.user;

    // Parse pagination parameters
    let page: u32 = params
        .get("page")
        .and_then(|p| p.parse().ok())
        .unwrap_or(1)
        .max(1);
    let per_page: u32 = 20; // FR-007: 20 events per page

    // Get total count for pagination
    let total_count = SecurityEventService::count_for_user(&state.db, viewer.id).await?;
    let total_pages = ((total_count as u32).saturating_sub(1) / per_page) + 1;

    // Get events for current page (FR-005: reverse chronological order)
    let events = SecurityEventService::list_for_user(&state.db, viewer.id, page, per_page).await?;

    // Convert to view models
    let events: Vec<SecurityEventView> = events.iter().map(SecurityEventView::from_event).collect();

    let pagination = PaginationInfo {
        current_page: page,
        total_pages,
        has_prev: page > 1,
        has_next: page < total_pages,
        prev_page: page.saturating_sub(1).max(1),
        next_page: (page + 1).min(total_pages),
    };

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    let template = SecurityEventsPageTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        events,
        pagination,
    };

    crate::core::render(&template)
}
