// Middleware module - auth, rate limiting

#![allow(unused_imports)]

pub mod auth;
pub mod rate_limit;

pub use auth::{
    clear_session_cookie, create_session_cookie, AuthUser, DbPool, OptionalAuthUser,
    SESSION_COOKIE_NAME, SESSION_EXPIRY_DAYS, SESSION_TOKEN_LENGTH,
};
