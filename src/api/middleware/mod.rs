// Middleware module - auth

pub mod auth;

pub use auth::{
    clear_session_cookie, create_session_cookie, AuthUser, OptionalAuthUser, SESSION_EXPIRY_DAYS,
};
