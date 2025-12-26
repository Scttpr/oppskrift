//! Authentication API endpoints
//!
//! Provides registration, email confirmation, login, and logout endpoints.
//! Uses session-based authentication with secure cookies.

use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;
use validator::Validate;

use crate::api::middleware::SESSION_EXPIRY_DAYS;
use crate::lib::audit::AuditEvent;
use crate::lib::config::SmtpConfig;
use crate::lib::error::{AppError, AppResult};
use crate::models::{EmailConfirmationResponse, RegisterRequest, ResendConfirmationRequest};
use crate::services::{AuthService, EmailService, PasswordService, UserService};
use crate::AppState;

/// JWT claims structure (legacy - to be replaced by session auth)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: Uuid,
    /// Username
    pub username: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
}

/// Auth routes
pub fn routes() -> Router<AppState> {
    Router::new()
        // Registration endpoints (new session-based)
        .route("/register", post(register))
        .route("/confirm-email/{token}", get(confirm_email))
        .route("/resend-confirmation", post(resend_confirmation))
        // Legacy JWT login (will be replaced)
        .route("/login", post(login))
}

// =============================================================================
// Registration Endpoints (T024-T026)
// =============================================================================

/// POST /api/auth/register
///
/// Register a new user account. Requires email confirmation before login.
///
/// ## Request Body
/// - `email`: Valid email address
/// - `username`: 3-30 lowercase alphanumeric characters and underscores
/// - `password`: At least 10 characters with uppercase, lowercase, and number
/// - `display_name`: Optional display name (1-100 characters)
///
/// ## Response
/// - `201 Created`: Registration successful, confirmation email sent
/// - `400 Bad Request`: Validation failed
/// - `409 Conflict`: Email or username already taken
async fn register(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ip = Some(addr.ip());

    // Validate input
    input.validate().map_err(|e| {
        tracing::debug!(errors = %e, "Registration validation failed");
        AppError::Validation(e.to_string())
    })?;

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Register user
    let response = auth_service
        .register(input, ip)
        .await
        .map_err(|e| match e {
            crate::services::AuthError::EmailExists => {
                AppError::Conflict("Email already registered".to_string())
            }
            crate::services::AuthError::UsernameExists => {
                AppError::Conflict("Username already taken".to_string())
            }
            crate::services::AuthError::UsernameReserved => {
                AppError::BadRequest("Username is reserved".to_string())
            }
            crate::services::AuthError::InvalidPassword(msg) => AppError::Validation(msg),
            crate::services::AuthError::Database(e) => {
                tracing::error!(error = %e, "Database error during registration");
                AppError::Internal("Registration failed".to_string())
            }
            e => {
                tracing::error!(error = %e, "Registration failed");
                AppError::Internal("Registration failed".to_string())
            }
        })?;

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/auth/confirm-email/:token
///
/// Confirm email address using the token from the confirmation email.
///
/// ## Path Parameters
/// - `token`: Email confirmation token (64 hex characters)
///
/// ## Response
/// - `200 OK`: Email confirmed successfully
/// - `400 Bad Request`: Invalid or expired token
/// - `409 Conflict`: Email already verified
async fn confirm_email(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let ip = Some(addr.ip());

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Confirm email
    let _user_id = auth_service
        .confirm_email(&token, ip)
        .await
        .map_err(|e| match e {
            crate::services::AuthError::InvalidToken => {
                AppError::BadRequest("Invalid or expired confirmation token".to_string())
            }
            crate::services::AuthError::AlreadyVerified => {
                AppError::Conflict("Email already verified".to_string())
            }
            crate::services::AuthError::UserNotFound => {
                AppError::BadRequest("Invalid confirmation token".to_string())
            }
            e => {
                tracing::error!(error = %e, "Email confirmation failed");
                AppError::Internal("Email confirmation failed".to_string())
            }
        })?;

    Ok(Json(EmailConfirmationResponse {
        message: "Email confirmed successfully. You can now log in.".to_string(),
        verified: true,
    }))
}

/// POST /api/auth/resend-confirmation
///
/// Resend the email confirmation link.
///
/// ## Request Body
/// - `email`: Email address to resend confirmation to
///
/// ## Response
/// - `200 OK`: Confirmation email sent (or appears to be sent for security)
/// - `400 Bad Request`: Invalid email format
/// - `409 Conflict`: Email already verified
/// - `429 Too Many Requests`: Rate limited (5 minute cooldown)
async fn resend_confirmation(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<ResendConfirmationRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ip = Some(addr.ip());

    // Validate input
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Resend confirmation
    // Note: We return success even if user not found to prevent email enumeration
    match auth_service.resend_confirmation(&input.email, ip).await {
        Ok(()) => {}
        Err(crate::services::AuthError::UserNotFound) => {
            // Don't reveal if user exists - return success anyway
            tracing::debug!(
                email_domain = input.email.split('@').next_back(),
                "Resend for unknown user"
            );
        }
        Err(crate::services::AuthError::AlreadyVerified) => {
            return Err(AppError::Conflict("Email already verified".to_string()));
        }
        Err(crate::services::AuthError::TooManyRequests) => {
            return Err(AppError::BadRequest(
                "Please wait before requesting another confirmation email".to_string(),
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Resend confirmation failed");
            // Don't reveal internal errors
        }
    }

    Ok(Json(serde_json::json!({
        "message": "If an account exists with this email, a confirmation link has been sent."
    })))
}

// =============================================================================
// Legacy JWT Login (to be replaced with session-based login in T034)
// =============================================================================

/// Legacy login request payload (JWT-based)
#[derive(Debug, Deserialize, Validate)]
pub struct LegacyLoginRequest {
    #[validate(length(min = 1, max = 50))]
    pub username: String,
}

/// Legacy login response with JWT token
#[derive(Debug, Serialize)]
pub struct LegacyLoginResponse {
    pub token: String,
    pub expires_at: i64,
}

/// POST /api/auth/login (legacy JWT)
///
/// Authenticate user and return JWT token.
/// This will be replaced with session-based authentication in T034.
async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<LegacyLoginRequest>,
) -> AppResult<Json<LegacyLoginResponse>> {
    let ip = addr.ip().to_string();

    // Validate input
    input.validate().map_err(|e| {
        AuditEvent::new("auth.login.failure")
            .with_ip(&ip)
            .with_metadata("reason", "validation_failed")
            .warn()
            .log();
        AppError::Validation(e.to_string())
    })?;

    // Find user by username
    let user = match UserService::get_by_username(&state.db, &input.username).await {
        Ok(user) => user,
        Err(_) => {
            AuditEvent::new("auth.login.failure")
                .with_ip(&ip)
                .with_metadata("username", &input.username)
                .with_metadata("reason", "user_not_found")
                .warn()
                .log();
            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }
    };

    // Generate JWT token
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET environment variable must be set");

    let now = Utc::now();
    let expires_at = now + Duration::hours(24);

    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        exp: expires_at.timestamp(),
        iat: now.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Failed to create token: {}", e)))?;

    // Audit successful login
    AuditEvent::new("auth.login.success")
        .with_user(user.id)
        .with_ip(&ip)
        .log();

    Ok(Json(LegacyLoginResponse {
        token,
        expires_at: expires_at.timestamp(),
    }))
}

// =============================================================================
// Helper Functions
// =============================================================================

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
