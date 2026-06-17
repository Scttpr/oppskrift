//! Rate limiting middleware
//!
//! Provides rate limiting for API endpoints using a custom sliding window implementation.
//! Supports IP-based and account-based rate limiting with configurable thresholds.

use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::PgPool;
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

/// Rate limit error response body
#[derive(Debug, Serialize)]
pub struct RateLimitErrorResponse {
    pub error: String,
    pub message: String,
    pub retry_after: u64,
}

/// Rate limit type for logging and response customization
#[derive(Debug, Clone, Copy)]
pub enum RateLimitType {
    /// Authentication endpoints (login, register, password reset)
    Auth,
    /// General API endpoints
    Api,
    /// Data export operations
    Export,
    /// Search operations
    Search,
    /// File upload operations
    Upload,
}

impl RateLimitType {
    /// Get human-readable message for rate limit exceeded
    pub fn message(&self, retry_after_seconds: u64) -> String {
        match self {
            Self::Auth => format!(
                "Too many failed login attempts. Please wait {} minutes before trying again.",
                retry_after_seconds.div_ceil(60)
            ),
            Self::Api => format!(
                "Too many requests. Please wait {} seconds before trying again.",
                retry_after_seconds
            ),
            Self::Export => format!(
                "You can only export data once per hour. Please wait {} minutes before trying again.",
                retry_after_seconds.div_ceil(60)
            ),
            Self::Search => format!(
                "Too many search requests. Please wait {} seconds before trying again.",
                retry_after_seconds
            ),
            Self::Upload => format!(
                "Too many uploads. Please wait {} seconds before trying again.",
                retry_after_seconds
            ),
        }
    }
}

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests allowed in window
    pub limit: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Rate limit type
    pub limit_type: RateLimitType,
}

impl RateLimitConfig {
    pub fn new(limit: u32, window_seconds: u64, limit_type: RateLimitType) -> Self {
        Self {
            limit,
            window_seconds,
            limit_type,
        }
    }

    /// Default auth rate limit: 5 attempts per 15 minutes
    pub fn default_auth() -> Self {
        Self::new(5, 15 * 60, RateLimitType::Auth)
    }

    /// Default API rate limit for unauthenticated users: 30 requests per minute
    pub fn default_api_unauthenticated() -> Self {
        Self::new(30, 60, RateLimitType::Api)
    }

    /// Default export rate limit: 1 per hour
    pub fn default_export() -> Self {
        Self::new(1, 60 * 60, RateLimitType::Export)
    }

    /// Default search rate limit: 10 per minute
    pub fn default_search() -> Self {
        Self::new(10, 60, RateLimitType::Search)
    }

    /// Default upload rate limit: 20 per 5 minutes
    pub fn default_upload() -> Self {
        Self::new(20, 5 * 60, RateLimitType::Upload)
    }

    /// Create auth config from environment
    pub fn from_env_auth() -> Self {
        let limit = std::env::var("RATE_LIMIT_AUTH_FAILED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        Self::new(limit, 15 * 60, RateLimitType::Auth)
    }

    /// Create API unauthenticated config from environment
    pub fn from_env_api_unauthenticated() -> Self {
        let limit = std::env::var("RATE_LIMIT_API_UNAUTHENTICATED")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);
        Self::new(limit, 60, RateLimitType::Api)
    }

    /// Create export config from environment
    pub fn from_env_export() -> Self {
        let limit = std::env::var("RATE_LIMIT_EXPORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        Self::new(limit, 60 * 60, RateLimitType::Export)
    }

    /// Create search config from environment
    pub fn from_env_search() -> Self {
        let limit = std::env::var("RATE_LIMIT_SEARCH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        Self::new(limit, 60, RateLimitType::Search)
    }

    /// Create upload config from environment
    pub fn from_env_upload() -> Self {
        let limit = std::env::var("RATE_LIMIT_UPLOAD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20);
        Self::new(limit, 5 * 60, RateLimitType::Upload)
    }
}

/// Simple sliding window rate limiter entry
#[derive(Debug, Clone)]
struct RateLimitEntry {
    /// Number of requests in the current window
    count: u32,
    /// Start of the current window
    window_start: Instant,
}

/// Simple IP-based rate limiter using sliding window
#[derive(Clone)]
pub struct SimpleRateLimiter {
    /// Map of IP -> rate limit entry
    entries: Arc<RwLock<HashMap<IpAddr, RateLimitEntry>>>,
    /// Maximum requests allowed
    limit: u32,
    /// Window duration
    window: Duration,
    /// Rate limit type
    limit_type: RateLimitType,
    /// Counter for triggering cleanup (atomic to avoid lock contention)
    request_count: Arc<std::sync::atomic::AtomicU64>,
}

/// How often to run cleanup (every N requests)
const CLEANUP_INTERVAL: u64 = 100;

impl SimpleRateLimiter {
    pub fn new(config: &RateLimitConfig) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            limit: config.limit,
            window: Duration::from_secs(config.window_seconds),
            limit_type: config.limit_type,
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Check if the request should be allowed AND increment counter
    /// Returns Ok(()) if allowed, Err(seconds_until_reset) if rate limited
    /// Use this for rate limiters where every request counts (e.g., API rate limits)
    pub fn check(&self, ip: IpAddr) -> Result<(), u64> {
        self.maybe_cleanup();

        let now = Instant::now();
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        let entry = entries.entry(ip).or_insert_with(|| RateLimitEntry {
            count: 0,
            window_start: now,
        });

        // Check if window has expired
        if now.duration_since(entry.window_start) >= self.window {
            // Reset the window
            entry.count = 1;
            entry.window_start = now;
            return Ok(());
        }

        // Check if under limit
        if entry.count < self.limit {
            entry.count += 1;
            return Ok(());
        }

        // Rate limited - calculate retry time
        let elapsed = now.duration_since(entry.window_start);
        let remaining = self.window.saturating_sub(elapsed);
        Err(remaining.as_secs() + 1)
    }

    /// Check if IP is currently rate limited WITHOUT incrementing counter
    /// Returns Ok(()) if under limit, Err(seconds_until_reset) if blocked
    /// Use this for pre-checking before processing a request
    pub fn is_limited(&self, ip: IpAddr) -> Result<(), u64> {
        self.maybe_cleanup();

        let now = Instant::now();
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());

        if let Some(entry) = entries.get(&ip) {
            // Check if window has expired
            if now.duration_since(entry.window_start) >= self.window {
                return Ok(()); // Window expired, not limited
            }

            // Check if at or over limit
            if entry.count >= self.limit {
                let elapsed = now.duration_since(entry.window_start);
                let remaining = self.window.saturating_sub(elapsed);
                return Err(remaining.as_secs() + 1);
            }
        }

        Ok(()) // Not in map or under limit
    }

    /// Record a failed attempt for an IP (increment counter)
    /// Use this after processing a request that failed (e.g., auth failure)
    pub fn record_failure(&self, ip: IpAddr) {
        let now = Instant::now();
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        let entry = entries.entry(ip).or_insert_with(|| RateLimitEntry {
            count: 0,
            window_start: now,
        });

        // Check if window has expired
        if now.duration_since(entry.window_start) >= self.window {
            // Reset the window
            entry.count = 1;
            entry.window_start = now;
        } else {
            entry.count += 1;
        }
    }

    /// Trigger cleanup occasionally to prevent memory growth
    fn maybe_cleanup(&self) {
        let count = self
            .request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % CLEANUP_INTERVAL == 0 {
            self.cleanup_expired(Instant::now());
        }
    }

    /// Remove expired entries to prevent memory growth
    fn cleanup_expired(&self, now: Instant) {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        entries.retain(|_ip, entry| now.duration_since(entry.window_start) < self.window);
    }

    /// Get the rate limit type
    pub fn limit_type(&self) -> RateLimitType {
        self.limit_type
    }

    /// Get the limit
    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// Get the window in seconds
    pub fn window_seconds(&self) -> u64 {
        self.window.as_secs()
    }
}

/// String-keyed rate limiter for account-based rate limiting
/// Similar to SimpleRateLimiter but keyed by String (email/username) instead of IP
#[derive(Clone)]
pub struct AccountRateLimiter {
    /// Map of account identifier -> rate limit entry
    entries: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
    /// Maximum requests allowed
    limit: u32,
    /// Window duration
    window: Duration,
    /// Counter for triggering cleanup
    request_count: Arc<std::sync::atomic::AtomicU64>,
}

impl AccountRateLimiter {
    pub fn new(limit: u32, window_seconds: u64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            limit,
            window: Duration::from_secs(window_seconds),
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Create with default auth account limits (10 per hour per account)
    pub fn default_auth_account() -> Self {
        Self::new(10, 60 * 60)
    }

    /// Create from environment variable
    pub fn from_env_auth_account() -> Self {
        let limit = std::env::var("RATE_LIMIT_AUTH_ACCOUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        Self::new(limit, 60 * 60)
    }

    /// Check if account is currently rate limited WITHOUT incrementing counter
    pub fn is_limited(&self, account: &str) -> Result<(), u64> {
        self.maybe_cleanup();

        let now = Instant::now();
        let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());

        if let Some(entry) = entries.get(account) {
            if now.duration_since(entry.window_start) >= self.window {
                return Ok(());
            }
            if entry.count >= self.limit {
                let elapsed = now.duration_since(entry.window_start);
                let remaining = self.window.saturating_sub(elapsed);
                return Err(remaining.as_secs() + 1);
            }
        }

        Ok(())
    }

    /// Record a failed attempt for an account
    pub fn record_failure(&self, account: &str) {
        let now = Instant::now();
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        let entry = entries
            .entry(account.to_lowercase())
            .or_insert_with(|| RateLimitEntry {
                count: 0,
                window_start: now,
            });

        if now.duration_since(entry.window_start) >= self.window {
            entry.count = 1;
            entry.window_start = now;
        } else {
            entry.count += 1;
        }
    }

    fn maybe_cleanup(&self) {
        let count = self
            .request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count % CLEANUP_INTERVAL == 0 {
            let now = Instant::now();
            let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
            entries.retain(|_k, entry| now.duration_since(entry.window_start) < self.window);
        }
    }

    pub fn limit(&self) -> u32 {
        self.limit
    }

    pub fn window_seconds(&self) -> u64 {
        self.window.as_secs()
    }
}

/// Rate limiter state shared across requests
#[derive(Clone)]
pub struct RateLimiterState {
    /// Auth endpoint rate limiter (per IP) - 5 failed attempts per 15 min
    pub auth: SimpleRateLimiter,
    /// Auth endpoint rate limiter (per account) - 10 failed attempts per hour across all IPs
    pub auth_account: AccountRateLimiter,
    /// General API rate limiter for unauthenticated requests (per IP)
    pub api_unauth: SimpleRateLimiter,
    /// Export rate limiter (per IP)
    pub export: SimpleRateLimiter,
    /// Search rate limiter (per IP)
    pub search: SimpleRateLimiter,
    /// Upload rate limiter (per IP)
    pub upload: SimpleRateLimiter,
    /// Database pool for logging security events
    pub db: PgPool,
    /// Trusted proxy CIDR ranges for X-Forwarded-For extraction
    pub trusted_proxies: Vec<IpNetwork>,
}

impl RateLimiterState {
    /// Create a new rate limiter state with default configuration
    pub fn new(db: PgPool) -> Self {
        let auth_config = RateLimitConfig::default_auth();
        let api_unauth_config = RateLimitConfig::default_api_unauthenticated();
        let export_config = RateLimitConfig::default_export();
        let search_config = RateLimitConfig::default_search();
        let upload_config = RateLimitConfig::default_upload();

        let trusted_proxies = parse_trusted_proxies();

        Self {
            auth: SimpleRateLimiter::new(&auth_config),
            auth_account: AccountRateLimiter::default_auth_account(),
            api_unauth: SimpleRateLimiter::new(&api_unauth_config),
            export: SimpleRateLimiter::new(&export_config),
            search: SimpleRateLimiter::new(&search_config),
            upload: SimpleRateLimiter::new(&upload_config),
            db,
            trusted_proxies,
        }
    }

    /// Create a rate limiter state from environment configuration
    pub fn from_env(db: PgPool) -> Self {
        let auth_config = RateLimitConfig::from_env_auth();
        let api_unauth_config = RateLimitConfig::from_env_api_unauthenticated();
        let export_config = RateLimitConfig::from_env_export();
        let search_config = RateLimitConfig::from_env_search();
        let upload_config = RateLimitConfig::from_env_upload();

        let trusted_proxies = parse_trusted_proxies();

        Self {
            auth: SimpleRateLimiter::new(&auth_config),
            auth_account: AccountRateLimiter::from_env_auth_account(),
            api_unauth: SimpleRateLimiter::new(&api_unauth_config),
            export: SimpleRateLimiter::new(&export_config),
            search: SimpleRateLimiter::new(&search_config),
            upload: SimpleRateLimiter::new(&upload_config),
            db,
            trusted_proxies,
        }
    }
}

/// Parse TRUSTED_PROXIES environment variable into CIDR ranges
fn parse_trusted_proxies() -> Vec<IpNetwork> {
    std::env::var("TRUSTED_PROXIES")
        .ok()
        .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
        .unwrap_or_default()
}

/// Extract client IP from request, respecting trusted proxies
pub fn extract_client_ip(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<SocketAddr>>,
    trusted_proxies: &[IpNetwork],
) -> Option<IpAddr> {
    // First, try to get the direct connection IP
    let direct_ip = connect_info.map(|ci| ci.0.ip());

    // If we have trusted proxies configured and direct IP is from a trusted proxy,
    // use X-Forwarded-For header
    if let Some(direct) = direct_ip {
        let is_trusted = trusted_proxies.iter().any(|net| net.contains(direct));

        if is_trusted {
            // Parse X-Forwarded-For header
            if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
                // Take the first (leftmost) IP, which is the original client
                if let Some(client_ip) = xff.split(',').next().and_then(|s| s.trim().parse().ok()) {
                    return Some(client_ip);
                }
            }
        }
    }

    // Fall back to direct connection IP
    direct_ip
}

/// Extract client IP from headers only (for middleware without ConnectInfo)
/// This is used in production behind a reverse proxy.
///
/// Security: Only trusts X-Forwarded-For if the request came through a trusted proxy.
/// If no trusted proxies are configured, headers are NOT trusted (prevents spoofing).
/// Returns None if IP cannot be determined - caller should fail open.
pub fn extract_client_ip_from_headers(
    headers: &HeaderMap,
    trusted_proxies: &[IpNetwork],
) -> Option<IpAddr> {
    // Only trust forwarding headers if we have trusted proxies configured
    // This prevents IP spoofing via X-Forwarded-For from untrusted sources
    if !trusted_proxies.is_empty() {
        // Try X-Forwarded-For header
        if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            // Take the first (leftmost) IP, which is the original client
            if let Some(client_ip) = xff.split(',').next().and_then(|s| s.trim().parse().ok()) {
                return Some(client_ip);
            }
        }

        // Try X-Real-IP header (used by some proxies like nginx)
        if let Some(real_ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
            if let Ok(ip) = real_ip.trim().parse() {
                return Some(ip);
            }
        }
    }

    // No IP could be determined from headers
    // Caller should fail open (allow request) when this returns None
    None
}

/// Create a 429 Too Many Requests response
pub fn rate_limit_response(limit_type: RateLimitType, retry_after_seconds: u64) -> Response {
    let body = RateLimitErrorResponse {
        error: "rate_limit_exceeded".to_string(),
        message: limit_type.message(retry_after_seconds),
        retry_after: retry_after_seconds,
    };

    let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response();

    // Add Retry-After header
    if let Ok(value) = HeaderValue::from_str(&retry_after_seconds.to_string()) {
        response.headers_mut().insert("Retry-After", value);
    }

    response
}

/// Caps in-flight best-effort rate-limit log writes so a request flood cannot
/// spawn unbounded DB-writing tasks.
static LOG_PERMITS: Semaphore = Semaphore::const_new(128);

/// Spawn a best-effort, non-blocking rate-limit log write, bounded by `LOG_PERMITS`.
/// Events are dropped (not logged) once the in-flight cap is reached.
#[allow(clippy::too_many_arguments)]
fn spawn_log_event(
    db: PgPool,
    ip: Option<IpAddr>,
    user_id: Option<uuid::Uuid>,
    endpoint: String,
    limit_type: RateLimitType,
    retry_after: u64,
    limit: u32,
    window_seconds: u64,
) {
    let Ok(permit) = LOG_PERMITS.try_acquire() else {
        return;
    };
    tokio::spawn(async move {
        let _permit = permit;
        log_rate_limit_event(
            &db,
            ip,
            user_id,
            &endpoint,
            limit_type,
            retry_after,
            limit,
            window_seconds,
        )
        .await;
    });
}

/// Log a rate limit event to the security_events table
#[allow(clippy::too_many_arguments)]
pub async fn log_rate_limit_event(
    db: &PgPool,
    ip: Option<IpAddr>,
    user_id: Option<uuid::Uuid>,
    endpoint: &str,
    limit_type: RateLimitType,
    retry_after: u64,
    limit: u32,
    window_seconds: u64,
) {
    let metadata = serde_json::json!({
        "endpoint": endpoint,
        "limit_type": format!("{:?}", limit_type).to_lowercase(),
        "retry_after": retry_after,
        "limit": limit,
        "window_seconds": window_seconds
    });

    let ip_str = ip.map(|i| i.to_string());

    let result = sqlx::query(
        r#"
        INSERT INTO security_events (user_id, event_type, ip_address, metadata)
        VALUES ($1, 'rate_limit_exceeded'::security_event_type, $2::inet, $3)
        "#,
    )
    .bind(user_id)
    .bind(&ip_str)
    .bind(&metadata)
    .execute(db)
    .await;

    if let Err(e) = result {
        tracing::error!(error = %e, "Failed to log rate limit event");
    }
}

/// Rate limiting layer for authentication endpoints
#[derive(Clone)]
pub struct AuthRateLimitLayer {
    state: RateLimiterState,
}

impl AuthRateLimitLayer {
    /// Create a new auth rate limit layer
    pub fn new(state: RateLimiterState) -> Self {
        Self { state }
    }
}

impl<S> tower::Layer<S> for AuthRateLimitLayer {
    type Service = AuthRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthRateLimitService {
            inner,
            state: self.state.clone(),
        }
    }
}

/// Rate limiting service for authentication endpoints
#[derive(Clone)]
pub struct AuthRateLimitService<S> {
    inner: S,
    state: RateLimiterState,
}

impl<S> tower::Service<Request<Body>> for AuthRateLimitService<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let state = self.state.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let headers = request.headers();
            let endpoint = request.uri().path().to_string();

            // Try to extract ConnectInfo from request extensions (set by into_make_service_with_connect_info)
            let connect_info = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .cloned();

            // Extract client IP using both headers and ConnectInfo
            let ip = extract_client_ip(headers, connect_info.as_ref(), &state.trusted_proxies);

            // If we can't determine IP, fail open (allow request)
            let Some(client_ip) = ip else {
                tracing::warn!("Could not determine client IP for rate limiting, failing open");
                return inner.call(request).await;
            };

            // Pre-check: Is this IP already rate limited?
            // This prevents processing requests from already-blocked IPs
            if let Err(retry_after) = state.auth.is_limited(client_ip) {
                // Already at limit - block without incrementing counter
                spawn_log_event(
                    state.db.clone(),
                    Some(client_ip),
                    None,
                    endpoint.clone(),
                    RateLimitType::Auth,
                    retry_after,
                    state.auth.limit(),
                    state.auth.window_seconds(),
                );

                return Ok(rate_limit_response(RateLimitType::Auth, retry_after));
            }

            // Process the request
            let response = inner.call(request).await?;

            // Post-check: Did authentication fail?
            // Only count failures (401 Unauthorized) against rate limit
            // This implements FR-001: "rate limit FAILED authentication attempts"
            let status = response.status();
            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                // Record this as a failed attempt
                state.auth.record_failure(client_ip);

                // Check if this failure pushed them over the limit
                // (for logging purposes - the response is already generated)
                if let Err(retry_after) = state.auth.is_limited(client_ip) {
                    spawn_log_event(
                        state.db.clone(),
                        Some(client_ip),
                        None,
                        endpoint.clone(),
                        RateLimitType::Auth,
                        retry_after,
                        state.auth.limit(),
                        state.auth.window_seconds(),
                    );
                }
            }

            Ok(response)
        })
    }
}

/// Rate limiting layer that counts ALL requests (not just failures)
/// Use this for endpoints like registration and password reset where
/// we want to prevent spam regardless of outcome
#[derive(Clone)]
pub struct AllRequestsRateLimitLayer {
    state: RateLimiterState,
}

impl AllRequestsRateLimitLayer {
    pub fn new(state: RateLimiterState) -> Self {
        Self { state }
    }
}

impl<S> tower::Layer<S> for AllRequestsRateLimitLayer {
    type Service = AllRequestsRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AllRequestsRateLimitService {
            inner,
            state: self.state.clone(),
        }
    }
}

/// Rate limiting service that counts ALL requests
#[derive(Clone)]
pub struct AllRequestsRateLimitService<S> {
    inner: S,
    state: RateLimiterState,
}

impl<S> tower::Service<Request<Body>> for AllRequestsRateLimitService<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let state = self.state.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let headers = request.headers();
            let endpoint = request.uri().path().to_string();

            // Try to extract ConnectInfo from request extensions
            let connect_info = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .cloned();

            // Extract client IP using both headers and ConnectInfo
            let ip = extract_client_ip(headers, connect_info.as_ref(), &state.trusted_proxies);

            // If we can't determine IP, fail open (allow request)
            let Some(client_ip) = ip else {
                tracing::warn!("Could not determine client IP for rate limiting, failing open");
                return inner.call(request).await;
            };

            // Check rate limit AND increment counter (counts all requests)
            match state.auth.check(client_ip) {
                Ok(()) => inner.call(request).await,
                Err(retry_after) => {
                    // Log the rate limit event asynchronously
                    spawn_log_event(
                        state.db.clone(),
                        Some(client_ip),
                        None,
                        endpoint.clone(),
                        RateLimitType::Auth,
                        retry_after,
                        state.auth.limit(),
                        state.auth.window_seconds(),
                    );

                    Ok(rate_limit_response(RateLimitType::Auth, retry_after))
                }
            }
        })
    }
}

// =============================================================================
// Tower Layers for specialized rate limiting
// =============================================================================

/// Rate limiting layer for export operations (1/hour)
#[derive(Clone)]
pub struct ExportRateLimitLayer {
    state: RateLimiterState,
}

impl ExportRateLimitLayer {
    pub fn new(state: RateLimiterState) -> Self {
        Self { state }
    }
}

impl<S> tower::Layer<S> for ExportRateLimitLayer {
    type Service = ExportRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ExportRateLimitService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ExportRateLimitService<S> {
    inner: S,
    state: RateLimiterState,
}

impl<S> tower::Service<Request<Body>> for ExportRateLimitService<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let state = self.state.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let headers = request.headers();
            let endpoint = request.uri().path().to_string();
            let connect_info = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .cloned();

            let ip = extract_client_ip(headers, connect_info.as_ref(), &state.trusted_proxies);

            let Some(client_ip) = ip else {
                tracing::warn!("Could not determine client IP for rate limiting, failing open");
                return inner.call(request).await;
            };

            match state.export.check(client_ip) {
                Ok(()) => inner.call(request).await,
                Err(retry_after) => {
                    spawn_log_event(
                        state.db.clone(),
                        Some(client_ip),
                        None,
                        endpoint.clone(),
                        RateLimitType::Export,
                        retry_after,
                        state.export.limit(),
                        state.export.window_seconds(),
                    );

                    Ok(rate_limit_response(RateLimitType::Export, retry_after))
                }
            }
        })
    }
}

/// Rate limiting layer for search operations (10/min)
#[derive(Clone)]
pub struct SearchRateLimitLayer {
    state: RateLimiterState,
}

impl SearchRateLimitLayer {
    pub fn new(state: RateLimiterState) -> Self {
        Self { state }
    }
}

impl<S> tower::Layer<S> for SearchRateLimitLayer {
    type Service = SearchRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SearchRateLimitService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct SearchRateLimitService<S> {
    inner: S,
    state: RateLimiterState,
}

impl<S> tower::Service<Request<Body>> for SearchRateLimitService<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let state = self.state.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let headers = request.headers();
            let endpoint = request.uri().path().to_string();
            let connect_info = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .cloned();

            let ip = extract_client_ip(headers, connect_info.as_ref(), &state.trusted_proxies);

            let Some(client_ip) = ip else {
                tracing::warn!("Could not determine client IP for rate limiting, failing open");
                return inner.call(request).await;
            };

            match state.search.check(client_ip) {
                Ok(()) => inner.call(request).await,
                Err(retry_after) => {
                    spawn_log_event(
                        state.db.clone(),
                        Some(client_ip),
                        None,
                        endpoint.clone(),
                        RateLimitType::Search,
                        retry_after,
                        state.search.limit(),
                        state.search.window_seconds(),
                    );

                    Ok(rate_limit_response(RateLimitType::Search, retry_after))
                }
            }
        })
    }
}

/// Rate limiting layer for upload operations (20/5min)
#[derive(Clone)]
pub struct UploadRateLimitLayer {
    state: RateLimiterState,
}

impl UploadRateLimitLayer {
    pub fn new(state: RateLimiterState) -> Self {
        Self { state }
    }
}

impl<S> tower::Layer<S> for UploadRateLimitLayer {
    type Service = UploadRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        UploadRateLimitService {
            inner,
            state: self.state.clone(),
        }
    }
}

#[derive(Clone)]
pub struct UploadRateLimitService<S> {
    inner: S,
    state: RateLimiterState,
}

impl<S> tower::Service<Request<Body>> for UploadRateLimitService<S>
where
    S: tower::Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let state = self.state.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let headers = request.headers();
            let endpoint = request.uri().path().to_string();
            let connect_info = request
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .cloned();

            let ip = extract_client_ip(headers, connect_info.as_ref(), &state.trusted_proxies);

            let Some(client_ip) = ip else {
                tracing::warn!("Could not determine client IP for rate limiting, failing open");
                return inner.call(request).await;
            };

            match state.upload.check(client_ip) {
                Ok(()) => inner.call(request).await,
                Err(retry_after) => {
                    spawn_log_event(
                        state.db.clone(),
                        Some(client_ip),
                        None,
                        endpoint.clone(),
                        RateLimitType::Upload,
                        retry_after,
                        state.upload.limit(),
                        state.upload.window_seconds(),
                    );

                    Ok(rate_limit_response(RateLimitType::Upload, retry_after))
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default_auth() {
        let config = RateLimitConfig::default_auth();
        assert_eq!(config.limit, 5);
        assert_eq!(config.window_seconds, 15 * 60);
    }

    #[test]
    fn test_rate_limit_config_default_api_unauthenticated() {
        let config = RateLimitConfig::default_api_unauthenticated();
        assert_eq!(config.limit, 30);
        assert_eq!(config.window_seconds, 60);
    }

    #[test]
    fn test_rate_limit_config_default_export() {
        let config = RateLimitConfig::default_export();
        assert_eq!(config.limit, 1);
        assert_eq!(config.window_seconds, 60 * 60);
    }

    #[test]
    fn test_rate_limit_type_message() {
        let auth_msg = RateLimitType::Auth.message(900);
        assert!(auth_msg.contains("15 minutes"));

        let api_msg = RateLimitType::Api.message(60);
        assert!(api_msg.contains("60 seconds"));

        let export_msg = RateLimitType::Export.message(2700);
        assert!(export_msg.contains("45 minutes"));
    }

    #[test]
    fn test_extract_client_ip_direct() {
        let headers = HeaderMap::new();
        let addr = SocketAddr::from(([192, 168, 1, 100], 8080));
        let connect_info = ConnectInfo(addr);

        let ip = extract_client_ip(&headers, Some(&connect_info), &[]);
        assert_eq!(ip, Some(IpAddr::from([192, 168, 1, 100])));
    }

    #[test]
    fn test_extract_client_ip_with_trusted_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.50, 10.0.0.1".parse().unwrap());

        let addr = SocketAddr::from(([10, 0, 0, 1], 8080));
        let connect_info = ConnectInfo(addr);

        let trusted: Vec<IpNetwork> = vec!["10.0.0.0/8".parse().unwrap()];

        let ip = extract_client_ip(&headers, Some(&connect_info), &trusted);
        assert_eq!(ip, Some(IpAddr::from([203, 0, 113, 50])));
    }

    #[test]
    fn test_extract_client_ip_untrusted_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.50".parse().unwrap());

        let addr = SocketAddr::from(([192, 168, 1, 100], 8080));
        let connect_info = ConnectInfo(addr);

        // Empty trusted proxies list
        let ip = extract_client_ip(&headers, Some(&connect_info), &[]);
        // Should use direct IP since proxy is not trusted
        assert_eq!(ip, Some(IpAddr::from([192, 168, 1, 100])));
    }

    #[test]
    fn test_simple_rate_limiter() {
        let config = RateLimitConfig::new(3, 60, RateLimitType::Auth);
        let limiter = SimpleRateLimiter::new(&config);
        let ip: IpAddr = "192.168.1.1".parse().unwrap();

        // First 3 requests should be allowed
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_ok());
        assert!(limiter.check(ip).is_ok());

        // 4th request should be rate limited
        assert!(limiter.check(ip).is_err());
    }

    #[test]
    fn test_simple_rate_limiter_different_ips() {
        let config = RateLimitConfig::new(2, 60, RateLimitType::Auth);
        let limiter = SimpleRateLimiter::new(&config);
        let ip1: IpAddr = "192.168.1.1".parse().unwrap();
        let ip2: IpAddr = "192.168.1.2".parse().unwrap();

        // Each IP has its own limit
        assert!(limiter.check(ip1).is_ok());
        assert!(limiter.check(ip1).is_ok());
        assert!(limiter.check(ip1).is_err()); // IP1 limited

        assert!(limiter.check(ip2).is_ok()); // IP2 still allowed
        assert!(limiter.check(ip2).is_ok());
        assert!(limiter.check(ip2).is_err()); // IP2 limited
    }
}
