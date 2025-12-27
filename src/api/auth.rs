//! Authentication API endpoints
//!
//! Provides registration, email confirmation, login, and logout endpoints.
//! Uses session-based authentication with secure cookies.

use axum::{
    extract::{ConnectInfo, Path, State},
    http::{header::SET_COOKIE, StatusCode},
    response::{AppendHeaders, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use validator::Validate;

use crate::api::middleware::{
    clear_session_cookie, create_session_cookie, AuthUser, SESSION_EXPIRY_DAYS,
};
use crate::lib::config::SmtpConfig;
use crate::lib::error::AppError;
use crate::models::{
    Complete2FALoginRequest, EmailConfirmationResponse, ForgotPasswordRequest,
    ForgotPasswordResponse, LoginRequest, LoginResponse, LogoutResponse, RegisterRequest,
    ResendConfirmationRequest, ResetPasswordRequest, ResetPasswordResponse, UserProfile,
};
use crate::services::{AuthError, AuthService, EmailService, LoginResult, PasswordService};
use crate::AppState;

/// Auth routes
pub fn routes() -> Router<AppState> {
    Router::new()
        // Registration endpoints
        .route("/register", post(register))
        .route("/confirm-email/{token}", get(confirm_email))
        .route("/resend-confirmation", post(resend_confirmation))
        // Session-based auth
        .route("/login", post(login))
        .route("/logout", post(logout))
        // 2FA verification
        .route("/2fa/verify", post(verify_2fa))
        // Password recovery
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
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
// Login/Logout Endpoints (T034-T035)
// =============================================================================

/// User agent header name
const USER_AGENT_HEADER: &str = "User-Agent";

/// POST /api/auth/login
///
/// Authenticate user with email and password. Sets session cookie on success.
///
/// ## Request Body
/// - `email`: User's email address
/// - `password`: User's password
/// - `totp_code`: Optional TOTP code (required if 2FA is enabled)
///
/// ## Response
/// - `200 OK`: Login successful, session cookie set
/// - `401 Unauthorized`: Invalid credentials
/// - `403 Forbidden`: Account locked or email not verified
/// - `422 Unprocessable Entity`: 2FA required
async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(input): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ip = Some(addr.ip());
    let user_agent = headers
        .get(USER_AGENT_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Validate input
    input.validate().map_err(|e| {
        tracing::debug!(errors = %e, "Login validation failed");
        AppError::Validation(e.to_string())
    })?;

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Attempt login
    let result = auth_service
        .login(&input.email, &input.password, ip, user_agent.clone(), None)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials => {
                AppError::Unauthorized("Invalid email or password".to_string())
            }
            AuthError::AccountLocked(until) => {
                AppError::Forbidden(format!("Account locked until {}", until))
            }
            AuthError::EmailNotVerified => {
                AppError::Forbidden("Please verify your email before logging in".to_string())
            }
            e => {
                tracing::error!(error = %e, "Login failed");
                AppError::Internal("Login failed".to_string())
            }
        })?;

    match result {
        LoginResult::Success {
            user_id,
            token,
            expires_at,
            ..
        } => {
            // Get user profile for response
            let user = crate::services::UserService::get_by_id(&state.db, user_id)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to get user after login");
                    AppError::Internal("Login failed".to_string())
                })?;

            let profile = UserProfile::from(user);

            // Set session cookie
            let cookie = create_session_cookie(&token, SESSION_EXPIRY_DAYS);

            Ok((
                StatusCode::OK,
                AppendHeaders([(SET_COOKIE, cookie)]),
                Json(serde_json::json!({
                    "user": profile,
                    "expires_at": expires_at,
                    "requires_2fa": false
                })),
            )
                .into_response())
        }
        LoginResult::TwoFactorRequired { partial_token } => {
            // Return a response with the partial token for 2FA completion
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "requires_2fa": true,
                    "partial_token": partial_token,
                    "message": "Two-factor authentication required"
                })),
            )
                .into_response())
        }
    }
}

/// POST /api/auth/logout
///
/// Terminate the current session and clear the session cookie.
///
/// ## Response
/// - `200 OK`: Logout successful
/// - `401 Unauthorized`: Not authenticated
async fn logout(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    // Create auth service
    let auth_service = create_auth_service(&state);

    // Logout the user
    auth_service
        .logout(auth_user.id, auth_user.session_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Logout failed");
            AppError::Internal("Logout failed".to_string())
        })?;

    // Clear session cookie
    let cookie = clear_session_cookie();

    Ok((
        AppendHeaders([(SET_COOKIE, cookie)]),
        Json(LogoutResponse {
            message: "Logged out successfully".to_string(),
        }),
    ))
}

// =============================================================================
// 2FA Verification Endpoint
// =============================================================================

/// POST /api/auth/2fa/verify
///
/// Complete login with 2FA verification. Called after initial login returns `requires_2fa: true`.
///
/// ## Request Body
/// - `partial_token`: Token from initial login response (64 hex characters)
/// - `totp_code`: TOTP code from authenticator app (6 digits)
///
/// ## Response
/// - `200 OK`: Login successful, session cookie set
/// - `400 Bad Request`: Invalid or expired token
/// - `401 Unauthorized`: Invalid TOTP code
async fn verify_2fa(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<Complete2FALoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ip = Some(addr.ip());

    // Validate input
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Complete 2FA login
    let result = auth_service
        .complete_2fa_login(&input.partial_token, &input.totp_code, ip, None)
        .await
        .map_err(|e| match e {
            AuthError::InvalidToken => {
                AppError::BadRequest("Invalid or expired verification token".to_string())
            }
            AuthError::InvalidTwoFactorCode => {
                AppError::Unauthorized("Invalid two-factor code".to_string())
            }
            e => {
                tracing::error!(error = %e, "2FA verification failed");
                AppError::Internal("2FA verification failed".to_string())
            }
        })?;

    match result {
        LoginResult::Success {
            user_id,
            token,
            expires_at,
            ..
        } => {
            // Get user profile for response
            let user = crate::services::UserService::get_by_id(&state.db, user_id)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to get user after 2FA verification");
                    AppError::Internal("Login failed".to_string())
                })?;

            let profile = UserProfile::from(user);

            // Set session cookie
            let cookie = create_session_cookie(&token, SESSION_EXPIRY_DAYS);

            Ok((
                AppendHeaders([(SET_COOKIE, cookie)]),
                Json(LoginResponse {
                    user: profile,
                    expires_at,
                }),
            ))
        }
        LoginResult::TwoFactorRequired { .. } => {
            // This shouldn't happen after 2FA verification
            Err(AppError::Internal("Unexpected 2FA state".to_string()))
        }
    }
}

// =============================================================================
// Password Recovery Endpoints (T045-T046)
// =============================================================================

/// POST /api/auth/forgot-password
///
/// Request a password reset link. Always returns success to prevent email enumeration.
///
/// ## Request Body
/// - `email`: Email address to send reset link to
///
/// ## Response
/// - `200 OK`: Reset email sent (or appears to be sent for security)
/// - `400 Bad Request`: Invalid email format
async fn forgot_password(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<ForgotPasswordRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ip = Some(addr.ip());

    // Validate input
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Request password reset
    // Always returns success to prevent email enumeration
    if let Err(e) = auth_service.forgot_password(&input.email, ip).await {
        tracing::error!(error = %e, "Password reset request failed");
        // Don't reveal internal errors
    }

    Ok(Json(ForgotPasswordResponse::success()))
}

/// POST /api/auth/reset-password
///
/// Reset password using the token from the reset email.
///
/// ## Request Body
/// - `token`: Password reset token (64 hex characters)
/// - `new_password`: New password (at least 10 characters)
///
/// ## Response
/// - `200 OK`: Password reset successfully
/// - `400 Bad Request`: Invalid token or weak password
async fn reset_password(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<ResetPasswordRequest>,
) -> Result<impl IntoResponse, AppError> {
    let ip = Some(addr.ip());

    // Validate input
    input
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Create auth service
    let auth_service = create_auth_service(&state);

    // Reset password
    auth_service
        .reset_password(&input.token, &input.new_password, ip)
        .await
        .map_err(|e| match e {
            AuthError::InvalidToken => {
                AppError::BadRequest("Invalid or expired reset token".to_string())
            }
            AuthError::InvalidPassword(msg) => AppError::Validation(msg),
            e => {
                tracing::error!(error = %e, "Password reset failed");
                AppError::Internal("Password reset failed".to_string())
            }
        })?;

    Ok(Json(ResetPasswordResponse::success()))
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
