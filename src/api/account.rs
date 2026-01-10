//! Account API endpoints
//!
//! Provides endpoints for authenticated users to manage their account.
//! All endpoints require a valid session.

use axum::{
    extract::{ConnectInfo, Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use std::net::SocketAddr;
use validator::Validate;

use crate::api::middleware::AuthUser;
use crate::core::audit::AuditEvent;
use crate::core::config::SmtpConfig;
use crate::core::error::AppError;
use crate::core::{RequestContext, RequestId};
use crate::models::{
    CancelDeletionResponse, ChangeEmailRequest, ChangeEmailResponse, ChangePasswordRequest,
    ChangePasswordResponse, DeleteAccountRequest, DeletionScheduledResponse, RecoveryCodesResponse,
    RecoveryCodesStatus, RegenerateRecoveryCodesRequest, SessionListResponse,
    TwoFactorEnabledResponse, TwoFactorSetupResponse, TwoFactorStatusResponse, UserProfile,
};
use crate::models::{DisableTwoFactorRequest, EnableTwoFactorRequest};
use crate::services::{
    AuthError, AuthService, EmailService, PasswordService, TotpError, TotpService, UserService,
};
use crate::AppState;

/// Session expiry in days (should match middleware)
const SESSION_EXPIRY_DAYS: u32 = 7;

/// Account routes - all require authentication
pub fn routes() -> Router<AppState> {
    Router::new()
        // Profile
        .route("/profile", get(get_profile))
        // Security info
        .route("/security", get(get_security_info))
        .route("/security-events", get(get_security_events))
        // Password & Email
        .route("/change-password", post(change_password))
        .route("/change-email", post(change_email))
        // Sessions
        .route("/sessions", get(list_sessions))
        .route("/sessions/{session_id}", delete(revoke_session))
        // 2FA
        .route("/2fa/setup", post(setup_2fa))
        .route("/2fa/enable", post(enable_2fa))
        .route("/2fa/disable", post(disable_2fa))
        .route("/2fa/status", get(get_2fa_status))
        .route("/2fa/recovery-codes", get(get_recovery_codes_status))
        .route("/2fa/recovery-codes", post(regenerate_recovery_codes))
        // Account deletion
        .route("/delete", post(request_deletion))
        .route("/cancel-deletion", post(cancel_deletion))
}

// =============================================================================
// Profile Endpoint
// =============================================================================

/// GET /api/account/profile
///
/// Get the authenticated user's profile.
async fn get_profile(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<UserProfile>, AppError> {
    let user = UserService::get_by_id(&state.db, auth_user.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, user_id = %auth_user.id, "Failed to get user profile");
            AppError::Internal("Failed to get profile".to_string())
        })?;

    Ok(Json(UserProfile::from(user)))
}

/// Security information response
#[derive(Debug, serde::Serialize)]
pub struct SecurityInfo {
    pub active_sessions_count: i64,
    pub two_factor_enabled: bool,
    pub recovery_codes_remaining: u32,
    pub email_verified: bool,
    pub deletion_requested: bool,
    pub locked_until: Option<chrono::DateTime<chrono::Utc>>,
    pub email_notifications_enabled: bool,
}

/// GET /api/account/security
///
/// Get the authenticated user's security information.
async fn get_security_info(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<SecurityInfo>, AppError> {
    let user = UserService::get_by_id(&state.db, auth_user.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get user");
            AppError::Internal("Failed to get security info".to_string())
        })?;

    // Get session count
    let session_service =
        crate::services::SessionService::new(state.db.clone(), SESSION_EXPIRY_DAYS);
    let active_sessions_count = session_service
        .count_for_user(auth_user.id)
        .await
        .unwrap_or(0);

    // Get 2FA status
    let mut recovery_codes_remaining = 0u32;
    if user.totp_enabled {
        if let Ok(totp_service) = create_totp_service(&state) {
            if let Ok((_, rem, _)) = totp_service.get_recovery_codes_status(auth_user.id).await {
                recovery_codes_remaining = rem;
            }
        }
    }

    // Check lockout status
    let auth_service = create_auth_service(&state);
    let locked_until = auth_service
        .check_lockout(auth_user.id)
        .await
        .unwrap_or(None);

    // Check email notifications enabled
    let email_service = create_email_service();
    let email_notifications_enabled = email_service.is_enabled();

    Ok(Json(SecurityInfo {
        active_sessions_count,
        two_factor_enabled: user.totp_enabled,
        recovery_codes_remaining,
        email_verified: user.email_verified,
        deletion_requested: user.deletion_requested_at.is_some(),
        locked_until,
        email_notifications_enabled,
    }))
}

/// Security event record for API responses (T083)
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct SecurityEvent {
    pub id: uuid::Uuid,
    pub event_type: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Security event view for template display (T004)
#[derive(Debug, Clone, serde::Serialize)]
pub struct SecurityEventView {
    pub id: uuid::Uuid,
    pub event_type: String,
    pub event_icon: String,
    pub ip_address: String,
    pub device_info: String,
    pub timestamp: String,
    pub metadata_summary: String,
}

impl SecurityEventView {
    /// Create a view from SecurityEvent with human-readable formatting
    pub fn from_event(event: &SecurityEvent) -> Self {
        Self {
            id: event.id,
            event_type: Self::format_event_type(&event.event_type),
            event_icon: Self::get_event_icon(&event.event_type),
            ip_address: event
                .ip_address
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            device_info: Self::parse_user_agent(event.user_agent.as_deref()),
            timestamp: crate::models::session::format_relative_time(event.created_at),
            metadata_summary: Self::summarize_metadata(event.metadata.as_ref()),
        }
    }

    /// Convert event type code to human-readable label
    fn format_event_type(event_type: &str) -> String {
        match event_type {
            "auth.login" => "Login".to_string(),
            "auth.login.failed" => "Failed login attempt".to_string(),
            "auth.logout" => "Logout".to_string(),
            "auth.password.change" => "Password changed".to_string(),
            "auth.password.reset" => "Password reset".to_string(),
            "auth.email.change" => "Email changed".to_string(),
            "auth.2fa.enable" => "2FA enabled".to_string(),
            "auth.2fa.disable" => "2FA disabled".to_string(),
            "auth.session.revoke" => "Session revoked".to_string(),
            "auth.recovery_code.used" => "Recovery code used".to_string(),
            "auth.deletion.request" => "Account deletion requested".to_string(),
            "auth.deletion.cancel" => "Account deletion cancelled".to_string(),
            other => other.replace(['.', '_'], " "),
        }
    }

    /// Get SVG icon path for event type
    fn get_event_icon(event_type: &str) -> String {
        match event_type {
            "auth.login" | "auth.logout" => "M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15m3 0l3-3m0 0l-3-3m3 3H9".to_string(),
            "auth.login.failed" => "M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z".to_string(),
            "auth.password.change" | "auth.password.reset" => "M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z".to_string(),
            "auth.email.change" => "M21.75 6.75v10.5a2.25 2.25 0 01-2.25 2.25h-15a2.25 2.25 0 01-2.25-2.25V6.75m19.5 0A2.25 2.25 0 0019.5 4.5h-15a2.25 2.25 0 00-2.25 2.25m19.5 0v.243a2.25 2.25 0 01-1.07 1.916l-7.5 4.615a2.25 2.25 0 01-2.36 0L3.32 8.91a2.25 2.25 0 01-1.07-1.916V6.75".to_string(),
            "auth.2fa.enable" | "auth.2fa.disable" => "M9 12.75L11.25 15 15 9.75m-3-7.036A11.959 11.959 0 013.598 6 11.99 11.99 0 003 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285z".to_string(),
            "auth.session.revoke" => "M6 18L18 6M6 6l12 12".to_string(),
            "auth.recovery_code.used" => "M15.75 5.25a3 3 0 013 3m3 0a6 6 0 01-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1121.75 8.25z".to_string(),
            "auth.deletion.request" | "auth.deletion.cancel" => "M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0".to_string(),
            _ => "M11.25 11.25l.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z".to_string(),
        }
    }

    /// Parse user agent string to human-readable device info
    fn parse_user_agent(user_agent: Option<&str>) -> String {
        match user_agent {
            Some(ua) if !ua.is_empty() => {
                // Simple parsing - extract browser and OS
                let browser = if ua.contains("Firefox") {
                    "Firefox"
                } else if ua.contains("Chrome") && !ua.contains("Edg") {
                    "Chrome"
                } else if ua.contains("Safari") && !ua.contains("Chrome") {
                    "Safari"
                } else if ua.contains("Edg") {
                    "Edge"
                } else {
                    "Browser"
                };

                let os = if ua.contains("Windows") {
                    "Windows"
                } else if ua.contains("Mac OS") || ua.contains("Macintosh") {
                    "macOS"
                } else if ua.contains("Linux") {
                    "Linux"
                } else if ua.contains("Android") {
                    "Android"
                } else if ua.contains("iPhone") || ua.contains("iPad") {
                    "iOS"
                } else {
                    "Unknown OS"
                };

                format!("{} on {}", browser, os)
            }
            _ => "Unknown device".to_string(),
        }
    }

    /// Summarize metadata JSON to a one-line string
    fn summarize_metadata(metadata: Option<&serde_json::Value>) -> String {
        match metadata {
            Some(serde_json::Value::Object(map)) if !map.is_empty() => {
                let parts: Vec<String> = map
                    .iter()
                    .take(3) // Limit to 3 fields
                    .filter_map(|(k, v)| {
                        let value_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Bool(b) => b.to_string(),
                            serde_json::Value::Number(n) => n.to_string(),
                            _ => return None,
                        };
                        Some(format!("{}: {}", k, value_str))
                    })
                    .collect();
                if parts.is_empty() {
                    String::new()
                } else {
                    parts.join(", ")
                }
            }
            _ => String::new(),
        }
    }
}

/// Security events list response (T083)
#[derive(Debug, serde::Serialize)]
pub struct SecurityEventsResponse {
    pub events: Vec<SecurityEvent>,
    pub total: usize,
}

/// Query parameters for security events
#[derive(Debug, serde::Deserialize)]
pub struct SecurityEventsQuery {
    /// Maximum number of events to return (default: 50, max: 100)
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

/// GET /api/account/security-events (T083)
///
/// Get the authenticated user's security event log.
async fn get_security_events(
    State(state): State<AppState>,
    auth_user: AuthUser,
    axum::extract::Query(query): axum::extract::Query<SecurityEventsQuery>,
) -> Result<Json<SecurityEventsResponse>, AppError> {
    // Cap limit between 1 and 100
    let limit = query.limit.clamp(1, 100);

    let events = sqlx::query_as::<_, SecurityEvent>(
        r#"
        SELECT id, event_type::text as event_type, ip_address::text as ip_address, user_agent, metadata, created_at
        FROM security_events
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(auth_user.id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to get security events");
        AppError::Internal("Failed to get security events".to_string())
    })?;

    let total = events.len();
    Ok(Json(SecurityEventsResponse { events, total }))
}

// =============================================================================
// Password & Email Endpoints (T058-T059)
// =============================================================================

/// POST /api/account/change-password
///
/// Change the authenticated user's password.
/// Invalidates all other sessions for security.
async fn change_password(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth_user: AuthUser,
    Json(input): Json<ChangePasswordRequest>,
) -> Result<Json<ChangePasswordResponse>, AppError> {
    let ctx = create_request_context(
        addr,
        request_id.as_ref().map(|e| &e.0),
        Some(auth_user.session_id),
    );

    // Validate input
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Change password
    let sessions_revoked = auth_service
        .change_password(
            auth_user.id,
            auth_user.session_id,
            &input.current_password,
            &input.new_password,
            &ctx,
        )
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials => {
                AppError::Unauthorized("Current password is incorrect".to_string())
            }
            AuthError::InvalidPassword(msg) => AppError::Validation(msg),
            e => {
                tracing::error!(error = %e, "Password change failed");
                AppError::Internal("Password change failed".to_string())
            }
        })?;

    Ok(Json(ChangePasswordResponse::success(sessions_revoked)))
}

/// POST /api/account/change-email
///
/// Request an email address change.
/// Sends confirmation to the new email.
async fn change_email(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth_user: AuthUser,
    Json(input): Json<ChangeEmailRequest>,
) -> Result<Json<ChangeEmailResponse>, AppError> {
    let ctx = create_request_context(
        addr,
        request_id.as_ref().map(|e| &e.0),
        Some(auth_user.session_id),
    );

    // Validate input
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Request email change
    auth_service
        .change_email(auth_user.id, &input.new_email, &input.password, &ctx)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials => {
                AppError::Unauthorized("Password is incorrect".to_string())
            }
            AuthError::EmailExists => {
                AppError::Conflict("Email address is already in use".to_string())
            }
            AuthError::App(msg) if msg.contains("same as current") => {
                AppError::BadRequest("New email is same as current".to_string())
            }
            e => {
                tracing::error!(error = %e, "Email change failed");
                AppError::Internal("Email change failed".to_string())
            }
        })?;

    Ok(Json(ChangeEmailResponse::success()))
}

// =============================================================================
// Session Endpoints (T060-T061)
// =============================================================================

/// GET /api/account/sessions
///
/// List all active sessions for the authenticated user.
async fn list_sessions(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<SessionListResponse>, AppError> {
    let session_service =
        crate::services::SessionService::new(state.db.clone(), SESSION_EXPIRY_DAYS);

    let service_sessions = session_service
        .list_for_user(auth_user.id, Some(auth_user.session_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list sessions");
            AppError::Internal("Failed to list sessions".to_string())
        })?;

    // Convert service SessionInfo to model SessionInfo
    let sessions: Vec<crate::models::SessionInfo> = service_sessions
        .into_iter()
        .map(|s| crate::models::SessionInfo {
            id: s.id,
            device_info: s.device_info,
            ip_address: s.ip_address,
            last_activity: s.last_activity,
            created_at: s.created_at,
            is_current: s.is_current,
        })
        .collect();

    let total = sessions.len();
    Ok(Json(SessionListResponse { sessions, total }))
}

/// DELETE /api/account/sessions/:session_id
///
/// Revoke a specific session.
async fn revoke_session(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth_user: AuthUser,
    Path(session_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ctx = create_request_context(
        addr,
        request_id.as_ref().map(|e| &e.0),
        Some(auth_user.session_id),
    );

    // Prevent revoking current session
    if session_id == auth_user.session_id {
        return Err(AppError::BadRequest(
            "Cannot revoke current session. Use logout instead.".to_string(),
        ));
    }

    let session_service =
        crate::services::SessionService::new(state.db.clone(), SESSION_EXPIRY_DAYS);

    // Verify session belongs to user before revoking
    let sessions = session_service
        .list_for_user(auth_user.id, None)
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
        .with_user(auth_user.id)
        .with_context(&ctx)
        .with_metadata("session_id", &session_id.to_string())
        .persist(&state.db)
        .await;

    Ok(Json(serde_json::json!({
        "message": "Session revoked successfully"
    })))
}

// =============================================================================
// 2FA Endpoints (T062-T066)
// =============================================================================

/// POST /api/account/2fa/setup
///
/// Start 2FA setup. Returns QR code and secret.
async fn setup_2fa(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<TwoFactorSetupResponse>, AppError> {
    let totp_service = create_totp_service(&state)?;

    // Get user email
    let user = UserService::get_by_id(&state.db, auth_user.id)
        .await
        .map_err(|_| AppError::Internal("Failed to get user".to_string()))?;

    let email = user
        .email
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Email required for 2FA setup".to_string()))?;

    let response = totp_service
        .setup_2fa(auth_user.id, email)
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

    Ok(Json(response))
}

/// POST /api/account/2fa/enable
///
/// Enable 2FA after verifying TOTP code. Returns recovery codes.
async fn enable_2fa(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth_user: AuthUser,
    Json(input): Json<EnableTwoFactorRequest>,
) -> Result<Json<TwoFactorEnabledResponse>, AppError> {
    let ctx = create_request_context(
        addr,
        request_id.as_ref().map(|e| &e.0),
        Some(auth_user.session_id),
    );

    // Validate input
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let totp_service = create_totp_service(&state)?;

    let recovery_codes = totp_service
        .enable_2fa(auth_user.id, &input.totp_code, &ctx)
        .await
        .map_err(|e| match e {
            TotpError::InvalidCode => {
                AppError::BadRequest("Invalid TOTP code. Please try again.".to_string())
            }
            TotpError::NoPendingSetup => {
                AppError::BadRequest("No pending 2FA setup. Call /2fa/setup first.".to_string())
            }
            e => {
                tracing::error!(error = %e, "2FA enable failed");
                AppError::Internal("2FA enable failed".to_string())
            }
        })?;

    // Send 2FA enabled notification email
    let user = UserService::get_by_id(&state.db, auth_user.id).await.ok();
    if let Some(user) = user {
        if let Some(email) = &user.email {
            let email_service = create_email_service();
            if let Err(e) = email_service.send_2fa_enabled_notification(email).await {
                tracing::warn!(error = %e, "Failed to send 2FA enabled notification");
            }
        }
    }

    Ok(Json(TwoFactorEnabledResponse {
        message: "Two-factor authentication has been enabled.".to_string(),
        recovery_codes,
    }))
}

/// POST /api/account/2fa/disable
///
/// Disable 2FA. Requires password and TOTP/recovery code.
async fn disable_2fa(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth_user: AuthUser,
    Json(input): Json<DisableTwoFactorRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ctx = create_request_context(
        addr,
        request_id.as_ref().map(|e| &e.0),
        Some(auth_user.session_id),
    );

    // Validate input
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // First verify password
    let auth_service = create_auth_service(&state);
    let user = UserService::get_by_id(&state.db, auth_user.id)
        .await
        .map_err(|_| AppError::Internal("Failed to get user".to_string()))?;

    let password_hash = user
        .password_hash
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Password not set for this account".to_string()))?;

    let password_valid = auth_service
        .password_service()
        .verify(&input.password, password_hash)
        .map_err(|e| {
            tracing::error!(error = %e, "Password verification failed");
            AppError::Internal("Failed to verify password".to_string())
        })?;

    if !password_valid {
        return Err(AppError::Unauthorized("Password is incorrect".to_string()));
    }

    // Then verify TOTP/recovery code and disable
    let totp_service = create_totp_service(&state)?;

    // Get user email before disabling (for notification)
    let user_email = user.email.clone();

    totp_service
        .disable_2fa(auth_user.id, &input.code, &ctx)
        .await
        .map_err(|e| match e {
            TotpError::InvalidCode | TotpError::InvalidRecoveryCode => {
                AppError::BadRequest("Invalid code".to_string())
            }
            TotpError::NotEnabled => {
                AppError::BadRequest("Two-factor authentication is not enabled".to_string())
            }
            e => {
                tracing::error!(error = %e, "2FA disable failed");
                AppError::Internal("2FA disable failed".to_string())
            }
        })?;

    // Send 2FA disabled notification email
    if let Some(email) = &user_email {
        let email_service = create_email_service();
        if let Err(e) = email_service.send_2fa_disabled_notification(email).await {
            tracing::warn!(error = %e, "Failed to send 2FA disabled notification");
        }
    }

    Ok(Json(serde_json::json!({
        "message": "Two-factor authentication has been disabled."
    })))
}

/// GET /api/account/2fa/status
///
/// Get 2FA status.
async fn get_2fa_status(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<TwoFactorStatusResponse>, AppError> {
    let user = UserService::get_by_id(&state.db, auth_user.id)
        .await
        .map_err(|_| AppError::Internal("Failed to get user".to_string()))?;

    let mut remaining = 0u32;
    if user.totp_enabled {
        if let Ok(totp_service) = create_totp_service(&state) {
            if let Ok((_, rem, _)) = totp_service.get_recovery_codes_status(auth_user.id).await {
                remaining = rem;
            }
        }
    }

    Ok(Json(TwoFactorStatusResponse {
        enabled: user.totp_enabled,
        recovery_codes_remaining: remaining,
    }))
}

/// GET /api/account/2fa/recovery-codes
///
/// Get recovery codes status (not the codes themselves).
async fn get_recovery_codes_status(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<Json<RecoveryCodesStatus>, AppError> {
    let totp_service = create_totp_service(&state)?;

    let (total, remaining, generated_at) = totp_service
        .get_recovery_codes_status(auth_user.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get recovery codes status");
            AppError::Internal("Failed to get recovery codes status".to_string())
        })?;

    Ok(Json(RecoveryCodesStatus {
        total,
        remaining,
        generated_at,
    }))
}

/// POST /api/account/2fa/recovery-codes
///
/// Regenerate recovery codes. Invalidates existing codes.
async fn regenerate_recovery_codes(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth_user: AuthUser,
    Json(input): Json<RegenerateRecoveryCodesRequest>,
) -> Result<Json<RecoveryCodesResponse>, AppError> {
    let ctx = create_request_context(
        addr,
        request_id.as_ref().map(|e| &e.0),
        Some(auth_user.session_id),
    );

    // Verify password first
    let auth_service = create_auth_service(&state);
    let user = UserService::get_by_id(&state.db, auth_user.id)
        .await
        .map_err(|_| AppError::Internal("Failed to get user".to_string()))?;

    let password_hash = user
        .password_hash
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("Password not set for this account".to_string()))?;

    let password_valid = auth_service
        .password_service()
        .verify(&input.password, password_hash)
        .map_err(|e| {
            tracing::error!(error = %e, "Password verification failed");
            AppError::Internal("Failed to verify password".to_string())
        })?;

    if !password_valid {
        return Err(AppError::Unauthorized("Password is incorrect".to_string()));
    }

    let totp_service = create_totp_service(&state)?;

    let response = totp_service
        .regenerate_recovery_codes(auth_user.id, &ctx)
        .await
        .map_err(|e| match e {
            TotpError::NotEnabled => {
                AppError::BadRequest("Two-factor authentication is not enabled".to_string())
            }
            e => {
                tracing::error!(error = %e, "Recovery code regeneration failed");
                AppError::Internal("Failed to regenerate recovery codes".to_string())
            }
        })?;

    Ok(Json(response))
}

// =============================================================================
// Account Deletion Endpoints (T076-T077)
// =============================================================================

/// POST /api/account/delete
///
/// Request account deletion. Schedules deletion after 7-day grace period.
async fn request_deletion(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth_user: AuthUser,
    Json(input): Json<DeleteAccountRequest>,
) -> Result<Json<DeletionScheduledResponse>, AppError> {
    let ctx = create_request_context(
        addr,
        request_id.as_ref().map(|e| &e.0),
        Some(auth_user.session_id),
    );

    // Validate input
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Request deletion
    let scheduled_for = auth_service
        .request_deletion(auth_user.id, &input.password, &ctx)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials => {
                AppError::Unauthorized("Password is incorrect".to_string())
            }
            AuthError::App(msg) if msg.contains("already scheduled") => {
                AppError::Conflict("Account is already scheduled for deletion".to_string())
            }
            e => {
                tracing::error!(error = %e, "Deletion request failed");
                AppError::Internal("Deletion request failed".to_string())
            }
        })?;

    Ok(Json(DeletionScheduledResponse::new(scheduled_for)))
}

/// POST /api/account/cancel-deletion
///
/// Cancel a scheduled account deletion during the grace period.
async fn cancel_deletion(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth_user: AuthUser,
) -> Result<Json<CancelDeletionResponse>, AppError> {
    let ctx = create_request_context(
        addr,
        request_id.as_ref().map(|e| &e.0),
        Some(auth_user.session_id),
    );

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Cancel deletion
    auth_service
        .cancel_deletion(auth_user.id, &ctx)
        .await
        .map_err(|e| match e {
            AuthError::App(msg) if msg.contains("No deletion") => {
                AppError::BadRequest("No deletion is scheduled".to_string())
            }
            e => {
                tracing::error!(error = %e, "Cancel deletion failed");
                AppError::Internal("Cancel deletion failed".to_string())
            }
        })?;

    Ok(Json(CancelDeletionResponse::success()))
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a RequestContext from request components
fn create_request_context(
    addr: SocketAddr,
    request_id: Option<&RequestId>,
    session_id: Option<uuid::Uuid>,
) -> RequestContext {
    RequestContext::new()
        .with_ip(addr.ip())
        .maybe_request_id(request_id.map(|r| r.0))
        .maybe_session_id(session_id)
}

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

/// Create a TotpService instance from AppState
fn create_totp_service(state: &AppState) -> Result<TotpService, AppError> {
    TotpService::from_env(state.db.clone()).map_err(|e| {
        tracing::error!(error = %e, "Failed to create TOTP service");
        AppError::Internal("2FA service unavailable".to_string())
    })
}

/// Create an EmailService instance
fn create_email_service() -> EmailService {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let is_production = std::env::var("APP_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    let smtp_config = SmtpConfig::from_env(is_production);
    EmailService::new(smtp_config, base_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_are_configured() {
        let _router = routes();
    }
}
