// Middleware module - auth, rate limiting

pub mod auth;
pub mod rate_limit;

pub use auth::{AuthUser, Claims, OptionalAuthUser};
pub use rate_limit::{create_rate_limit_layer, RateLimitConfig};
