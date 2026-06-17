//! Account settings handlers (email change, account deletion)

use std::net::SocketAddr;

use askama::Template;
use axum::{
    extract::{ConnectInfo, State},
    response::{Html, Redirect},
    Form,
};
use serde::Deserialize;
use validator::Validate;

use super::{
    create_auth_service, create_request_context, generate_csrf, validate_csrf, CsrfOnlyForm,
};
use crate::api::middleware::AuthUser;
use crate::core::audit::AuditEvent;
use crate::core::error::AppResult;
use crate::core::helpers::mask_email;
use crate::core::request_id::RequestId;
use crate::models::DeletionContentChoice;
use crate::services::UserService;
use crate::AppState;

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
pub(crate) async fn account_page(
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

    crate::core::render(&template)
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
pub(crate) async fn email_change_page(
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

    crate::core::render(&template)
}

/// Email change handler (POST) (T035)
pub(crate) async fn email_change(
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

        return crate::core::render(&template);
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

    crate::core::render(&template)
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
pub(crate) async fn delete_account_page(
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

    crate::core::render(&template)
}

/// Delete account handler (POST) (T070)
pub(crate) async fn delete_account(
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
    UserService::set_deletion_content_choice(&state.db, auth.id, content_choice).await?;

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

            crate::core::render(&template)
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

            crate::core::render(&template)
        }
    }
}

/// Cancel deletion handler (POST) (T072)
pub(crate) async fn cancel_deletion(
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
