//! Security test helpers for OSK Principle IV compliance
//!
//! Provides utilities for security-focused testing including:
//! - Timing measurement helpers
//! - User creation helpers for security tests
//! - Rate limit testing utilities

use std::time::{Duration, Instant};

/// Measure the timing of an async operation
/// Returns the elapsed time
pub async fn measure_timing<F, Fut, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = f().await;
    (result, start.elapsed())
}

/// Calculate the average duration from a slice of durations
pub fn average_duration(durations: &[Duration]) -> Duration {
    if durations.is_empty() {
        return Duration::ZERO;
    }
    let total: Duration = durations.iter().sum();
    total / durations.len() as u32
}

/// Calculate the standard deviation of durations (in milliseconds)
pub fn std_deviation_ms(durations: &[Duration]) -> f64 {
    if durations.len() < 2 {
        return 0.0;
    }

    let mean = average_duration(durations).as_secs_f64() * 1000.0;
    let variance: f64 = durations
        .iter()
        .map(|d| {
            let diff = d.as_secs_f64() * 1000.0 - mean;
            diff * diff
        })
        .sum::<f64>()
        / (durations.len() - 1) as f64;

    variance.sqrt()
}

/// Check if two sets of timings are statistically similar
/// Uses a simple threshold-based comparison
/// Returns true if the difference between averages is less than threshold_ms
pub fn timings_are_similar(times_a: &[Duration], times_b: &[Duration], threshold_ms: u64) -> bool {
    let avg_a = average_duration(times_a);
    let avg_b = average_duration(times_b);

    let diff_ms = if avg_a > avg_b {
        (avg_a - avg_b).as_millis()
    } else {
        (avg_b - avg_a).as_millis()
    };

    diff_ms < threshold_ms as u128
}

/// Rate limit test result
#[derive(Debug)]
pub struct RateLimitTestResult {
    /// Number of successful requests before rate limiting
    pub successful_requests: u32,
    /// Whether rate limiting was triggered
    pub rate_limited: bool,
    /// The HTTP status code of the rate-limited response
    pub rate_limit_status: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_average_duration() {
        let durations = vec![
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(300),
        ];
        let avg = average_duration(&durations);
        assert_eq!(avg, Duration::from_millis(200));
    }

    #[test]
    fn test_average_duration_empty() {
        let avg = average_duration(&[]);
        assert_eq!(avg, Duration::ZERO);
    }

    #[test]
    fn test_std_deviation() {
        let durations = vec![
            Duration::from_millis(100),
            Duration::from_millis(100),
            Duration::from_millis(100),
        ];
        let std_dev = std_deviation_ms(&durations);
        assert!(std_dev < 0.001, "Identical values should have ~0 std dev");
    }

    #[test]
    fn test_timings_similar() {
        let times_a = vec![Duration::from_millis(100), Duration::from_millis(110)];
        let times_b = vec![Duration::from_millis(105), Duration::from_millis(115)];
        assert!(timings_are_similar(&times_a, &times_b, 20));
    }

    #[test]
    fn test_timings_not_similar() {
        let times_a = vec![Duration::from_millis(100)];
        let times_b = vec![Duration::from_millis(200)];
        assert!(!timings_are_similar(&times_a, &times_b, 50));
    }
}
