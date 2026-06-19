//! Privacy settings handlers (federation toggle, data export)

use std::net::SocketAddr;

use askama::Template;
use axum::{
    extract::{ConnectInfo, State},
    response::{Html, IntoResponse, Response},
    Form,
};

use super::{create_request_context, generate_csrf, validate_csrf, CsrfOnlyForm};
use crate::api::middleware::AuthUser;
use crate::core::audit::AuditEvent;
use crate::core::error::{AppError, AppResult};
use crate::core::request_id::RequestId;
use crate::services::UserService;
use crate::AppState;

/// Privacy page template (T019)
#[derive(Template)]
#[template(path = "settings/privacy.html")]
struct PrivacyPageTemplate {
    active_tab: &'static str,
    deletion_pending: bool,
    deletion_date: Option<String>,
    flash_success: Option<String>,
    flash_error: Option<String>,
    federation_enabled: bool,
    csrf_token: String,
}

/// Privacy settings page handler (GET) (T020)
pub(crate) async fn privacy_page(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let csrf_token = generate_csrf(&state, auth.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    let template = PrivacyPageTemplate {
        active_tab: "privacy",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        federation_enabled: user.federation_enabled,
        csrf_token,
    };

    crate::core::render(&template)
}

/// Federation toggle handler (POST) (T021)
///
/// HTMX endpoint to toggle ActivityPub federation on/off.
pub(crate) async fn toggle_federation(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(form): Form<CsrfOnlyForm>,
) -> AppResult<Html<String>> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    // Get current state and toggle
    let user = UserService::get_by_id(&state.db, auth.id).await?;
    let new_state = !user.federation_enabled;

    // Update federation status
    UserService::update_federation_enabled(&state.db, auth.id, new_state).await?;

    // Log the change
    let event_type = if new_state {
        "settings.federation.enable"
    } else {
        "settings.federation.disable"
    };
    AuditEvent::new(event_type)
        .with_user(auth.id)
        .with_context(&ctx)
        .persist(&state.db)
        .await;

    // Return updated toggle button for HTMX swap
    let csrf_token = generate_csrf(&state, auth.session_id);
    let html = format!(
        r#"<form
            method="POST"
            action="/settings/privacy/federation"
            hx-post="/settings/privacy/federation"
            hx-swap="outerHTML"
            hx-target="this"
        >
            <input type="hidden" name="_csrf" value="{}" />
            <button
                type="submit"
                class="relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 {}"
                role="switch"
                aria-checked="{}"
            >
                <span
                    class="pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out {}"
                ></span>
            </button>
        </form>"#,
        csrf_token,
        if new_state {
            "bg-primary-600"
        } else {
            "bg-gray-200 dark:bg-gray-700"
        },
        new_state,
        if new_state {
            "translate-x-5"
        } else {
            "translate-x-0"
        }
    );

    Ok(Html(html))
}

/// Export data handler (POST) (T024, T026)
///
/// Exports user's personal data as JSON download.
/// Rate limited to 1 export per hour (T028).
pub(crate) async fn export_data(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    auth: AuthUser,
    Form(form): Form<CsrfOnlyForm>,
) -> Result<Response, AppError> {
    use crate::services::{ExportRateLimitResult, ExportService};

    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, auth.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), auth.session_id);

    // Use advisory lock to prevent TOCTOU race condition on rate limit
    let lock_acquired = ExportService::try_acquire_lock(&state.db, auth.id).await?;
    if !lock_acquired {
        return Err(AppError::BadRequest(
            "Export already in progress. Please wait and try again.".to_string(),
        ));
    }

    // Check rate limit (T028): 1 export per hour
    match ExportService::check_rate_limit(&state.db, auth.id).await? {
        ExportRateLimitResult::RateLimited(wait_mins) => {
            return Err(AppError::BadRequest(format!(
                "Export rate limit exceeded. Please wait {} more minute(s).",
                wait_mins
            )));
        }
        ExportRateLimitResult::Allowed => {}
    }

    // Get user data
    let user = UserService::get_by_id(&state.db, auth.id).await?;

    // Check recipe count for async threshold (FR-015: >50 recipes = async)
    let recipe_count = ExportService::count_user_recipes(&state.db, auth.id).await?;

    // For now, we only implement sync export (T026)
    // TODO: Implement async export for >50 recipes in future iteration
    if recipe_count > 50 {
        return Err(AppError::BadRequest(
            "Large exports (>50 recipes) require async processing. Feature coming soon."
                .to_string(),
        ));
    }

    // Build export data using ExportService
    let export = ExportService::build_export(&state.db, &user).await?;

    // Log export event
    AuditEvent::new("account_export")
        .with_user(auth.id)
        .with_context(&ctx)
        .persist(&state.db)
        .await;

    // Build JSON response with download headers
    let json = serde_json::to_string_pretty(&export)
        .map_err(|e| AppError::Internal(format!("JSON serialization failed: {}", e)))?;

    let filename = format!(
        "oppskrift-export-{}-{}.json",
        user.username,
        chrono::Utc::now().format("%Y%m%d")
    );

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        json,
    )
        .into_response())
}
