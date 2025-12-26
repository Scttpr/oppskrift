use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use validator::Validate;

use crate::api::middleware::Claims;
use crate::lib::audit::AuditEvent;
use crate::lib::error::{AppError, AppResult};
use crate::services::UserService;
use crate::AppState;

/// Auth routes
pub fn routes() -> Router<AppState> {
    Router::new().route("/login", post(login))
}

/// Login request payload
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, max = 50))]
    pub username: String,
    // Note: In a real app, you'd have password here
    // For ActivityPub federation, auth might be OAuth or other mechanism
}

/// Login response with JWT token
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: i64,
}

/// POST /api/v1/auth/login
/// Authenticate user and return JWT token
async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(input): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
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

    // In a real app, verify password here
    // For now, this is a stub for ActivityPub federation testing

    // Generate JWT token
    let secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET environment variable must be set");

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

    Ok(Json(LoginResponse {
        token,
        expires_at: expires_at.timestamp(),
    }))
}
