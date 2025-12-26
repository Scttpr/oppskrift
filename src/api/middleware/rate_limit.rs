//! Rate limiting middleware using tower-governor
//!
//! Provides configurable rate limiting for API endpoints.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;

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

/// Create a rate limiting layer for API routes
pub fn create_rate_limit_layer(
    config: &RateLimitConfig,
) -> impl tower::Layer<axum::routing::Router> + Clone {
    let governor_config = std::sync::Arc::new(
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
