//! Profile settings handlers

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
    contains_dangerous_content, create_request_context, generate_csrf, sanitize_text, validate_csrf,
};
use crate::api::middleware::Viewer;
use crate::core::audit::AuditEvent;
use crate::core::error::AppResult;
use crate::core::helpers::mask_email;
use crate::core::request_id::RequestId;
use crate::models::{MeasurementPref, UpdateUser, User};
use crate::services::UserService;
use crate::AppState;

/// Redirect /settings to /settings/profile (T020)
pub(crate) async fn settings_redirect() -> Redirect {
    Redirect::to("/settings/profile")
}

// =============================================================================
// Profile View (User Story 1)
// =============================================================================

/// User profile view for display (T018)
///
/// Contains fields safe to display, with masked email for privacy.
pub(crate) struct ProfileView {
    pub(crate) display_name: String,
    pub(crate) username: String,
    pub(crate) masked_email: String,
    pub(crate) email_verified: bool,
    pub(crate) bio: Option<String>,
    avatar_url: Option<String>,
    measurement_pref: MeasurementPref,
    totp_enabled: bool,
    created_at: String,
}

impl ProfileView {
    pub(crate) fn from_user(user: &User) -> Self {
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
            created_at: crate::core::helpers::format_fr_date(&user.created_at),
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
pub(crate) async fn profile_page(viewer: Viewer) -> AppResult<Html<String>> {
    let user = viewer.user;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    let template = ProfileTemplate {
        active_tab: "profile",
        deletion_pending,
        deletion_date,
        flash_success: None,
        flash_error: None,
        profile: ProfileView::from_user(&user),
    };

    crate::core::render(&template)
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

    #[validate(length(
        min = 1,
        max = 100,
        message = "Le nom affiché doit comporter de 1 à 100 caractères"
    ))]
    pub display_name: String,

    #[validate(length(max = 500, message = "La bio doit comporter au plus 500 caractères"))]
    pub bio: Option<String>,

    #[validate(url(message = "L'URL de l'avatar doit être une URL valide"))]
    pub avatar_url: Option<String>,

    pub measurement_pref: String,
}

impl UpdateProfileForm {
    /// Sanitize input to reject HTML/script tags (T027 - RISK-004-002)
    pub(crate) fn sanitize(&mut self) {
        self.display_name = sanitize_text(&self.display_name);
        self.bio = self.bio.as_ref().map(|b| sanitize_text(b));
        // Convert empty avatar_url to None (empty string fails URL validation)
        self.avatar_url = self.avatar_url.as_ref().and_then(|url| {
            let trimmed = url.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
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
pub(crate) async fn profile_edit_page(
    State(state): State<AppState>,
    viewer: Viewer,
) -> AppResult<Html<String>> {
    let user = viewer.user;

    let deletion_pending = user.deletion_requested_at.is_some();
    let deletion_date = user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    // Generate CSRF token (T028)
    let csrf_token = generate_csrf(&state, viewer.session_id);

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

    crate::core::render(&template)
}

/// Profile update handler (POST) (T026)
pub(crate) async fn profile_update(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request_id: Option<axum::Extension<RequestId>>,
    viewer: Viewer,
    Form(mut form): Form<UpdateProfileForm>,
) -> AppResult<Html<String>> {
    // Validate CSRF token
    validate_csrf(&state, &form.csrf_token, viewer.session_id)?;

    let ctx = create_request_context(addr, request_id.as_ref().map(|e| &e.0), viewer.session_id);
    let user = viewer.user;

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
        let deletion_date = user
            .deletion_requested_at
            .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

        let csrf_token = generate_csrf(&state, viewer.session_id);

        let template = ProfileEditTemplate {
            active_tab: "profile",
            deletion_pending,
            deletion_date,
            flash_success: None,
            flash_error: Some("Corrige les erreurs ci-dessous".to_string()),
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

        return crate::core::render(&template);
    }

    // Update user profile
    let update_data = form.to_update_user();
    UserService::update(&state.db, viewer.id, update_data).await?;

    // Log profile update (T031 - RISK-004-005)
    AuditEvent::new("settings.profile.update")
        .with_user(viewer.id)
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
    let updated_user = UserService::get_by_id(&state.db, viewer.id).await?;

    let deletion_pending = updated_user.deletion_requested_at.is_some();
    let deletion_date = updated_user
        .deletion_requested_at
        .map(|dt| crate::core::helpers::format_fr_date(&(dt + chrono::Duration::days(30))));

    let template = ProfileTemplate {
        active_tab: "profile",
        deletion_pending,
        deletion_date,
        flash_success: Some("Profil mis à jour avec succès".to_string()),
        flash_error: None,
        profile: ProfileView::from_user(&updated_user),
    };

    crate::core::render(&template)
}
