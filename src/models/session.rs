//! Session models for authentication

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

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

/// Session list response
#[derive(Debug, Serialize)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionInfo>,
    pub total: usize,
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
