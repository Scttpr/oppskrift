use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::lib::audit::AuditEvent;

/// JWT claims structure
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

/// Authenticated user extracted from JWT
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub username: String,
}

impl From<Claims> for AuthUser {
    fn from(claims: Claims) -> Self {
        Self {
            id: claims.sub,
            username: claims.username,
        }
    }
}

/// Auth error response
#[derive(Debug, Serialize)]
struct AuthError {
    error: String,
    message: String,
}

/// Extract authenticated user from request
/// Returns 401 Unauthorized if no valid token is present
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get the Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(AuthError {
                        error: "unauthorized".to_string(),
                        message: "Missing authorization header".to_string(),
                    }),
                )
                    .into_response()
            })?;

        // Extract Bearer token
        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "unauthorized".to_string(),
                    message: "Invalid authorization header format".to_string(),
                }),
            )
                .into_response()
        })?;

        // Get JWT secret from environment - MUST be set, no fallback
        let secret =
            std::env::var("JWT_SECRET").expect("JWT_SECRET environment variable must be set");

        // Decode and validate JWT
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| {
            tracing::debug!("JWT validation failed: {:?}", e);

            // Audit invalid token attempt
            AuditEvent::new("auth.token.invalid")
                .with_metadata("reason", &e.to_string())
                .warn()
                .log();

            (
                StatusCode::UNAUTHORIZED,
                Json(AuthError {
                    error: "unauthorized".to_string(),
                    message: "Invalid or expired token".to_string(),
                }),
            )
                .into_response()
        })?;

        Ok(AuthUser::from(token_data.claims))
    }
}

/// Optional authenticated user - doesn't fail if no token is present
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_user = AuthUser::from_request_parts(parts, state).await.ok();
        Ok(OptionalAuthUser(auth_user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_user_from_claims() {
        let claims = Claims {
            sub: Uuid::new_v4(),
            username: "testuser".to_string(),
            exp: 0,
            iat: 0,
        };

        let auth_user: AuthUser = claims.clone().into();
        assert_eq!(auth_user.id, claims.sub);
        assert_eq!(auth_user.username, claims.username);
    }
}
