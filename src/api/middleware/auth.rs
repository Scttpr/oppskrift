//! Authentication middleware
//!
//! Supports both session cookie authentication (for browsers) and
//! Bearer token authentication (for API clients).

use axum::{
    extract::FromRequestParts,
    http::{header::COOKIE, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::services::ServiceFactory;

/// Session cookie name
pub const SESSION_COOKIE_NAME: &str = "oppskrift_session";

/// Session token length in hex chars (64 chars = 32 bytes)
pub const SESSION_TOKEN_LENGTH: usize = 64;

/// Session expiry in days
pub const SESSION_EXPIRY_DAYS: u32 = 30;

/// Authenticated user extracted from session
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub session_id: Uuid,
}

/// Auth error response
#[derive(Debug, Serialize)]
struct AuthError {
    error: String,
    code: String,
}

impl AuthError {
    fn unauthorized(message: &str, code: &str) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(Self {
                error: message.to_string(),
                code: code.to_string(),
            }),
        )
            .into_response()
    }
}

/// Extract session token from cookie header
fn extract_session_token(cookie_header: &str) -> Option<&str> {
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some(token) = cookie.strip_prefix(&format!("{}=", SESSION_COOKIE_NAME)) {
            // Validate token format (hex string of correct length)
            if token.len() == SESSION_TOKEN_LENGTH && token.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(token);
            }
        }
    }
    None
}

/// Extract session token from Authorization: Bearer header (for API clients)
fn extract_bearer_token(auth_header: &str) -> Option<&str> {
    let token = auth_header.strip_prefix("Bearer ")?;
    // Validate token format
    if token.len() == SESSION_TOKEN_LENGTH && token.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(token);
    }
    None
}

/// Extract authenticated user from request
/// Supports both session cookie and Bearer token authentication
impl FromRequestParts<crate::AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        // Try to get token from cookie first, then Authorization header
        let token = parts
            .headers
            .get(COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(extract_session_token)
            .or_else(|| {
                parts
                    .headers
                    .get(axum::http::header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .and_then(extract_bearer_token)
            })
            .ok_or_else(|| AuthError::unauthorized("Authentication required", "AUTH_REQUIRED"))?;

        // Validate session
        let session_service = ServiceFactory::create_session_service(state.db.clone());

        let (session_id, user_id) = session_service.validate(token).await.map_err(|e| {
            tracing::debug!("Session validation failed: {:?}", e);
            AuthError::unauthorized("Invalid or expired session", "INVALID_SESSION")
        })?;

        Ok(AuthUser {
            id: user_id,
            session_id,
        })
    }
}

/// Optional authenticated user - doesn't fail if no token is present
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

impl FromRequestParts<crate::AppState> for OptionalAuthUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await.ok();
        Ok(OptionalAuthUser(auth_user))
    }
}

/// Helper to create a session cookie header value
pub fn create_session_cookie(token: &str, max_age_days: u32) -> String {
    let max_age_seconds = max_age_days as i64 * 24 * 60 * 60;
    format!(
        "{}={}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={}",
        SESSION_COOKIE_NAME, token, max_age_seconds
    )
}

/// Helper to create a cookie that clears the session
pub fn clear_session_cookie() -> String {
    format!(
        "{}=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0",
        SESSION_COOKIE_NAME
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_token_valid() {
        let cookie = format!("{}={}; other=value", SESSION_COOKIE_NAME, "a".repeat(64));
        let token = extract_session_token(&cookie);
        assert_eq!(token, Some("a".repeat(64).as_str()));
    }

    #[test]
    fn test_extract_session_token_invalid_length() {
        let cookie = format!("{}=tooshort", SESSION_COOKIE_NAME);
        let token = extract_session_token(&cookie);
        assert!(token.is_none());
    }

    #[test]
    fn test_extract_session_token_missing() {
        let cookie = "other=value; another=thing";
        let token = extract_session_token(cookie);
        assert!(token.is_none());
    }

    #[test]
    fn test_extract_bearer_token_valid() {
        let header = format!("Bearer {}", "b".repeat(64));
        let token = extract_bearer_token(&header);
        assert_eq!(token, Some("b".repeat(64).as_str()));
    }

    #[test]
    fn test_extract_bearer_token_invalid() {
        let token = extract_bearer_token("Bearer invalid");
        assert!(token.is_none());
    }

    #[test]
    fn test_create_session_cookie() {
        let cookie = create_session_cookie("test_token", 30);
        assert!(cookie.contains("oppskrift_session=test_token"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("Secure"));
        assert!(cookie.contains("SameSite=Strict"));
    }

    #[test]
    fn test_clear_session_cookie() {
        let cookie = clear_session_cookie();
        assert!(cookie.contains("Max-Age=0"));
    }
}
