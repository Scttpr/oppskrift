// Middleware module - auth, security, and rate limiting

pub mod auth;
pub mod rate_limit;
pub mod security;

pub use auth::{
    clear_session_cookie, create_session_cookie, AuthUser, OptionalAuthUser, SESSION_EXPIRY_DAYS,
};
pub use rate_limit::{
    api_rate_limit_middleware, export_rate_limit_middleware, search_rate_limit_middleware,
    upload_rate_limit_middleware, AuthRateLimitLayer, RateLimiterState,
};
pub use security::security_headers;
