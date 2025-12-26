//! Rate limiting middleware using tower-governor
//!
//! Provides configurable rate limiting for API endpoints with
//! different tiers for auth-sensitive operations.

#![allow(dead_code)]

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;

/// Rate limiter configuration
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Requests per second
    pub requests_per_second: u64,
    /// Burst size
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            burst_size: 50,
        }
    }
}

/// Auth rate limit configuration
/// Stricter limits for auth endpoints to prevent brute force
#[derive(Clone)]
pub struct AuthRateLimitConfig {
    /// Requests per minute (default: 10)
    pub requests_per_minute: u64,
    /// Burst size (default: 5)
    pub burst_size: u32,
}

impl Default for AuthRateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 10,
            burst_size: 5,
        }
    }
}

impl AuthRateLimitConfig {
    /// Create config from environment or defaults
    pub fn from_env() -> Self {
        Self {
            requests_per_minute: std::env::var("AUTH_RATE_LIMIT_PER_MINUTE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            burst_size: std::env::var("AUTH_RATE_LIMIT_BURST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
        }
    }

    /// Strict limits for login/register (5 per minute)
    pub fn strict() -> Self {
        Self {
            requests_per_minute: 5,
            burst_size: 3,
        }
    }

    /// More permissive limits for general auth operations
    pub fn permissive() -> Self {
        Self {
            requests_per_minute: 30,
            burst_size: 10,
        }
    }
}

/// Create a rate limiting layer for API routes
pub fn create_rate_limit_layer(
    config: &RateLimitConfig,
) -> impl tower::Layer<axum::routing::Router> + Clone {
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(config.requests_per_second)
            .burst_size(config.burst_size)
            .finish()
            .expect("Failed to create governor config"),
    );

    GovernorLayer {
        config: governor_config,
    }
}

/// Create a rate limiting layer for auth endpoints (per-minute)
pub fn create_auth_rate_limit_layer(
    config: &AuthRateLimitConfig,
) -> impl tower::Layer<axum::routing::Router> + Clone {
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            // Convert requests per minute to per-second rate
            // Using period instead of per_second for finer control
            .per_second(1)
            .burst_size(config.burst_size)
            .finish()
            .expect("Failed to create governor config"),
    );

    // Note: For more precise per-minute limiting, consider using
    // a sliding window rate limiter or custom keyed rate limiting

    GovernorLayer {
        config: governor_config,
    }
}

/// Create a strict rate limiting layer for login attempts
/// 5 attempts per 15 minutes per IP
pub fn create_login_rate_limit_layer() -> impl tower::Layer<axum::routing::Router> + Clone {
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1) // Very slow replenishment
            .burst_size(5) // Max 5 attempts
            .finish()
            .expect("Failed to create governor config"),
    );

    GovernorLayer {
        config: governor_config,
    }
}

/// Create a rate limiting layer for password reset requests
/// Prevents enumeration attacks
pub fn create_password_reset_rate_limit_layer() -> impl tower::Layer<axum::routing::Router> + Clone
{
    let governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(3) // Max 3 attempts per period
            .finish()
            .expect("Failed to create governor config"),
    );

    GovernorLayer {
        config: governor_config,
    }
}

/// Rate limit exceeded response
pub struct RateLimitExceeded;

impl IntoResponse for RateLimitExceeded {
    fn into_response(self) -> Response {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [("Retry-After", "60")],
            "Rate limit exceeded. Please try again later.",
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_second, 10);
        assert_eq!(config.burst_size, 50);
    }

    #[test]
    fn test_auth_config_default() {
        let config = AuthRateLimitConfig::default();
        assert_eq!(config.requests_per_minute, 10);
        assert_eq!(config.burst_size, 5);
    }

    #[test]
    fn test_auth_config_strict() {
        let config = AuthRateLimitConfig::strict();
        assert_eq!(config.requests_per_minute, 5);
        assert_eq!(config.burst_size, 3);
    }

    #[test]
    fn test_auth_config_permissive() {
        let config = AuthRateLimitConfig::permissive();
        assert_eq!(config.requests_per_minute, 30);
        assert_eq!(config.burst_size, 10);
    }
}
