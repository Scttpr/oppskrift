//! Two-Factor Authentication (2FA) settings handlers

use std::net::SocketAddr;

use askama::Template;
use axum::{
    extract::{ConnectInfo, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;

use super::{
    create_auth_service, create_request_context, create_totp_service, generate_csrf, validate_csrf,
};
use crate::api::middleware::Viewer;
use crate::core::error::{AppError, AppResult};
use crate::core::request_id::RequestId;
use crate::services::TotpError;
use crate::AppState;

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
pub(crate) async fn twofa_setup_page(
    State(state): State<AppState>,
    viewer: Viewer,
) -> AppResult<Html<String>> {
    let user = viewer.user;
    let csrf_token = generate_csrf(&state, viewer.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

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

        return crate::core::render(&template);
    }

    // Generate 2FA setup (QR code and secret)
    let totp_service = create_totp_service(&state)?;
    let email = user.email.as_ref().ok_or_else(|| {
        AppError::BadRequest("Une adresse e-mail est requise pour configurer la 2FA".to_string())
    })?;

    let setup = totp_service
        .setup_2fa(viewer.id, email)
        .await
        .map_err(|e| match e {
            TotpError::AlreadyEnabled => AppError::Conflict(
                "L'authentification à deux facteurs est déjà activée".to_string(),
            ),
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

    crate::core::render(&template)
}

/// 2FA enable handler (POST) - Verifies TOTP code and enables 2FA
pub(crate) async fn twofa_enable(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    viewer: Viewer,
    Form(form): Form<TwoFaEnableForm>,
) -> AppResult<Html<String>> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, viewer.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), viewer.session_id);
    let user = viewer.user;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    // Validate TOTP code format
    if form.totp_code.len() != 6 || !form.totp_code.chars().all(|c| c.is_ascii_digit()) {
        let csrf_token = generate_csrf(&state, viewer.session_id);
        let totp_service = create_totp_service(&state)?;
        let empty_email = String::new();
        let email = user.email.as_ref().unwrap_or(&empty_email);
        let setup = totp_service.setup_2fa(viewer.id, email).await.ok();

        let template = TwoFaSetupTemplate {
            active_tab: "security",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Saisis un code valide à 6 chiffres".to_string()),
            qr_code: setup.as_ref().map(|s| s.qr_code.clone()),
            secret: setup.as_ref().map(|s| s.secret.clone()),
            csrf_token,
            already_enabled: false,
        };

        return crate::core::render(&template);
    }

    let totp_service = create_totp_service(&state)?;

    match totp_service
        .enable_2fa(viewer.id, &form.totp_code, &ctx)
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

            crate::core::render(&template)
        }
        Err(e) => {
            let error_msg = match e {
                TotpError::InvalidCode => "Code invalide. Réessaie.".to_string(),
                TotpError::NoPendingSetup => "La configuration a expiré. Recommence.".to_string(),
                _ => "Échec de l'activation de la 2FA. Réessaie.".to_string(),
            };

            // Re-generate QR code for retry
            let empty_email = String::new();
            let email = user.email.as_ref().unwrap_or(&empty_email);
            let setup = totp_service.setup_2fa(viewer.id, email).await.ok();
            let csrf_token = generate_csrf(&state, viewer.session_id);

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

            crate::core::render(&template)
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
pub(crate) async fn twofa_recovery_page(
    State(state): State<AppState>,
    viewer: Viewer,
) -> AppResult<Html<String>> {
    let user = viewer.user;
    let csrf_token = generate_csrf(&state, viewer.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    // Check if 2FA is enabled
    if !user.totp_enabled {
        return Err(AppError::BadRequest(
            "L'authentification à deux facteurs n'est pas activée".to_string(),
        ));
    }

    let totp_service = create_totp_service(&state)?;
    let (total, remaining, generated_at) = totp_service
        .get_recovery_codes_status(viewer.id)
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
        generated_at: generated_at.map(|dt| crate::core::helpers::format_fr_datetime(&dt)),
        csrf_token,
        new_codes: None,
    };

    crate::core::render(&template)
}

/// 2FA regenerate recovery codes form
#[derive(Debug, Deserialize)]
pub struct TwoFaRegenerateForm {
    #[serde(rename = "_csrf")]
    pub csrf_token: String,
    pub password: String,
}

/// 2FA regenerate recovery codes handler (POST)
pub(crate) async fn twofa_regenerate_codes(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    viewer: Viewer,
    Form(form): Form<TwoFaRegenerateForm>,
) -> AppResult<Html<String>> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, viewer.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), viewer.session_id);
    let user = viewer.user;
    let csrf_token = generate_csrf(&state, viewer.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    // Verify password first
    let auth_service = create_auth_service(&state);
    let password_hash = user.password_hash.as_ref().ok_or_else(|| {
        AppError::BadRequest("Aucun mot de passe défini pour ce compte".to_string())
    })?;

    let password_valid = auth_service
        .password_service()
        .verify(&form.password, password_hash)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Password verification failed");
            AppError::Internal("Failed to verify password".to_string())
        })?;

    if !password_valid {
        let totp_service = create_totp_service(&state)?;
        let (total, remaining, generated_at) = totp_service
            .get_recovery_codes_status(viewer.id)
            .await
            .unwrap_or((8, 8, None));

        let template = TwoFaRecoveryTemplate {
            active_tab: "security",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Mot de passe incorrect".to_string()),
            codes_total: total,
            codes_remaining: remaining,
            generated_at: generated_at.map(|dt| crate::core::helpers::format_fr_datetime(&dt)),
            csrf_token,
            new_codes: None,
        };

        return crate::core::render(&template);
    }

    // Regenerate codes
    let totp_service = create_totp_service(&state)?;
    let response = totp_service
        .regenerate_recovery_codes(viewer.id, &ctx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to regenerate recovery codes");
            AppError::Internal("Failed to regenerate recovery codes".to_string())
        })?;

    let template = TwoFaRecoveryTemplate {
        active_tab: "security",
        deletion_pending,
        deletion_date,
        flash_success: Some("Codes de récupération régénérés avec succès".to_string()),
        flash_error: None,
        codes_total: 8,
        codes_remaining: 8,
        generated_at: Some(crate::core::helpers::format_fr_datetime(&chrono::Utc::now())),
        csrf_token,
        new_codes: Some(response.codes),
    };

    crate::core::render(&template)
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
pub(crate) async fn twofa_disable_page(
    State(state): State<AppState>,
    viewer: Viewer,
) -> AppResult<Html<String>> {
    let user = viewer.user;
    let csrf_token = generate_csrf(&state, viewer.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    // Check if 2FA is enabled
    if !user.totp_enabled {
        return Err(AppError::BadRequest(
            "L'authentification à deux facteurs n'est pas activée".to_string(),
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

    crate::core::render(&template)
}

/// 2FA disable handler (POST)
pub(crate) async fn twofa_disable(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    viewer: Viewer,
    Form(form): Form<TwoFaDisableForm>,
) -> Result<Response, AppError> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, viewer.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), viewer.session_id);
    let user = viewer.user;
    let csrf_token = generate_csrf(&state, viewer.session_id);

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    // Verify password first
    let auth_service = create_auth_service(&state);
    let password_hash = user.password_hash.as_ref().ok_or_else(|| {
        AppError::BadRequest("Aucun mot de passe défini pour ce compte".to_string())
    })?;

    let password_valid = auth_service
        .password_service()
        .verify(&form.password, password_hash)
        .await
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
            flash_error: Some("Mot de passe incorrect".to_string()),
            csrf_token,
        };

        return Ok(crate::core::render(&template)?.into_response());
    }

    // Verify TOTP/recovery code and disable
    let totp_service = create_totp_service(&state)?;

    match totp_service.disable_2fa(viewer.id, &form.code, &ctx).await {
        Ok(()) => {
            // Redirect to security page with success message
            Ok(Redirect::to("/settings/security").into_response())
        }
        Err(e) => {
            let error_msg = match e {
                TotpError::InvalidCode | TotpError::InvalidRecoveryCode => {
                    "Code invalide. Réessaie.".to_string()
                }
                TotpError::NotEnabled => {
                    "L'authentification à deux facteurs n'est pas activée".to_string()
                }
                _ => "Échec de la désactivation de la 2FA. Réessaie.".to_string(),
            };

            let template = TwoFaDisableTemplate {
                active_tab: "security",
                deletion_pending,
                deletion_date,
                flash_success: None,
                flash_error: Some(error_msg),
                csrf_token,
            };

            Ok(crate::core::render(&template)?.into_response())
        }
    }
}
