//! Session models for authentication
//!
//! Session tokens are stored hashed in the database. The raw token
//! is only known to the client (stored in HttpOnly cookie).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::net::IpAddr;
use uuid::Uuid;

/// Active session record from database
#[derive(Debug, Clone, FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub device_info: Option<String>,
    pub ip_address: Option<String>, // Stored as text from INET
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Session info for display to user (safe to expose)
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub id: Uuid,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub last_activity: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

/// Create new session (internal use)
#[derive(Debug)]
pub struct CreateSession {
    pub user_id: Uuid,
    pub token_hash: String,
    pub device_info: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub expires_at: DateTime<Utc>,
}

/// Session list response
#[derive(Debug, Serialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionInfo>,
    pub total: usize,
}

/// Revoke session request
#[derive(Debug, Deserialize)]
pub struct RevokeSessionRequest {
    pub session_id: Uuid,
}

/// Revoke all sessions response
#[derive(Debug, Serialize)]
pub struct RevokeAllSessionsResponse {
    pub revoked_count: u64,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_info_serialization() {
        let info = SessionInfo {
            id: Uuid::new_v4(),
            device_info: Some("Firefox on Linux".to_string()),
            ip_address: Some("192.168.1.1".to_string()),
            last_activity: Utc::now(),
            created_at: Utc::now(),
            is_current: true,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("is_current"));
        assert!(json.contains("Firefox"));
    }
}
