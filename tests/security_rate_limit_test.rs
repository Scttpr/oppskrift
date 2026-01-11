//! Rate limiting security tests (T066-T068)
//!
//! Tests for OSK Principle IV compliance - Rate Limiting
//! Verifies that authentication endpoints are rate limited to prevent brute force attacks.
//!
//! Important: Rate limiting only counts FAILED authentication attempts (401/403 responses).
//! Successful logins do not count against the rate limit.
//!
//! Run with: cargo test --test security_rate_limit_test

mod common;

use common::{run_test, run_test_with_rate_limiting};
use serde_json::json;

// =============================================================================
// Login Rate Limiting Tests (T066) - Using Rate-Limited Router
// =============================================================================

/// Test: Login endpoint should return 429 after exceeding FAILED attempts
///
/// This test uses the rate-limited router to verify end-to-end rate limiting.
/// Default limit: 5 FAILED attempts per 15 minutes per IP
/// Note: Only 401/403 responses count against the limit
#[tokio::test]
async fn test_login_rate_limit_middleware_blocks_after_failures() {
    run_test_with_rate_limiting(|ctx| async move {
        // Make 6 failed login attempts (wrong password = 401)
        // After 5 failures, the 6th should be blocked with 429
        let mut last_response = None;
        for i in 0..6 {
            let response = ctx
                .post(
                    "/api/v1/auth/login",
                    json!({
                        "email": format!("user{}@example.com", i),
                        "password": "WrongPassword123!"
                    }),
                )
                .await;

            last_response = Some(response);
        }

        let response = last_response.expect("Should have made requests");

        // 6th request should be rate limited (after 5 failures)
        assert_eq!(
            response.status, 429,
            "6th request should return 429 Too Many Requests after 5 failures. Got {} - {:?}",
            response.status, response.body
        );

        // Should have Retry-After header
        assert!(
            response.retry_after.is_some(),
            "Rate limited response should include Retry-After header"
        );

        // Response body should have expected structure
        assert_eq!(
            response.body.get("error").and_then(|v| v.as_str()),
            Some("rate_limit_exceeded"),
            "Response should have error: rate_limit_exceeded"
        );
    })
    .await;
}

/// Test: Rate limit response includes proper JSON body
#[tokio::test]
async fn test_login_rate_limit_response_body() {
    run_test_with_rate_limiting(|ctx| async move {
        // Exhaust rate limit
        for i in 0..6 {
            let response = ctx
                .post(
                    "/api/v1/auth/login",
                    json!({
                        "email": format!("bodytest{}@example.com", i),
                        "password": "WrongPassword123!"
                    }),
                )
                .await;

            if response.status == 429 {
                // Verify response body structure
                assert!(
                    response.body.get("error").is_some(),
                    "Response should have 'error' field"
                );
                assert!(
                    response.body.get("message").is_some(),
                    "Response should have 'message' field"
                );
                assert!(
                    response.body.get("retry_after").is_some(),
                    "Response should have 'retry_after' field"
                );
                return;
            }
        }

        panic!("Should have received 429 response");
    })
    .await;
}

// =============================================================================
// Registration Rate Limiting Tests (T067)
// =============================================================================

/// Test: Registration endpoint should be rate limited
#[tokio::test]
async fn test_registration_rate_limit_middleware() {
    run_test_with_rate_limiting(|ctx| async move {
        let mut rate_limited = false;
        let mut successful_requests = 0;

        // Try 7 registrations - should hit rate limit
        for i in 0..7 {
            let response = ctx
                .post(
                    "/api/v1/auth/register",
                    json!({
                        "email": format!("ratetest{}@example.com", i),
                        "username": format!("ratetest{}", i),
                        "password": "SecurePass123!@#"
                    }),
                )
                .await;

            if response.status == 429 {
                rate_limited = true;
                // Verify has proper response structure
                assert!(
                    response.retry_after.is_some(),
                    "Rate limited response should have Retry-After header"
                );
                break;
            }
            successful_requests += 1;
        }

        assert!(
            rate_limited,
            "Registration endpoint should be rate limited after {} successful requests",
            successful_requests
        );
    })
    .await;
}

// =============================================================================
// Password Reset Rate Limiting Tests (T068)
// =============================================================================

/// Test: Password reset endpoint should be rate limited
#[tokio::test]
async fn test_password_reset_rate_limit_middleware() {
    run_test_with_rate_limiting(|ctx| async move {
        let mut rate_limited = false;

        // Try many password reset requests - should hit rate limit
        for i in 0..7 {
            let response = ctx
                .post(
                    "/api/v1/auth/forgot-password",
                    json!({ "email": format!("resettest{}@example.com", i) }),
                )
                .await;

            if response.status == 429 {
                rate_limited = true;
                assert!(
                    response.retry_after.is_some(),
                    "Rate limited response should have Retry-After header"
                );
                break;
            }
        }

        assert!(
            rate_limited,
            "Password reset endpoint should be rate limited"
        );
    })
    .await;
}

// =============================================================================
// Security Event Logging Tests
// =============================================================================

/// Test: Rate limit events should be logged to security_events table
#[tokio::test]
async fn test_rate_limit_event_logged() {
    run_test_with_rate_limiting(|ctx| async move {
        // Clear any existing rate limit events for test IP
        let _ = sqlx::query("DELETE FROM security_events WHERE event_type = 'rate_limit_exceeded' AND ip_address = '127.0.0.1'")
            .execute(&ctx.db)
            .await;

        // Exhaust rate limit
        for i in 0..6 {
            let _ = ctx
                .post(
                    "/api/v1/auth/login",
                    json!({
                        "email": format!("logevent{}@example.com", i),
                        "password": "WrongPassword123!"
                    }),
                )
                .await;
        }

        // Give a moment for async logging
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Check that a rate limit event was logged
        let event_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM security_events WHERE event_type = 'rate_limit_exceeded' AND ip_address = '127.0.0.1'"
        )
        .fetch_one(&ctx.db)
        .await
        .expect("Failed to query security events");

        assert!(
            event_count > 0,
            "Rate limit exceeded event should be logged to security_events table"
        );
    })
    .await;
}

// =============================================================================
// Legacy Tests (Non-Rate-Limited Context for Backward Compatibility)
// =============================================================================

/// Test: Non-rate-limited context should allow unlimited requests
///
/// This verifies that test_app_router (used by most tests) doesn't have rate limiting,
/// ensuring test isolation.
#[tokio::test]
async fn test_non_rate_limited_context_allows_requests() {
    run_test(|ctx| async move {
        // Make many requests - none should be rate limited
        for i in 0..20 {
            let response = ctx
                .post(
                    "/api/v1/auth/login",
                    json!({
                        "email": format!("unlimited{}@example.com", i),
                        "password": "WrongPassword123!"
                    }),
                )
                .await;

            assert_ne!(
                response.status, 429,
                "Non-rate-limited context should not return 429"
            );
        }
    })
    .await;
}
