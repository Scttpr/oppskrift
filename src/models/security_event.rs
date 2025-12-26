//! Security event models for audit logging
//!
//! All security-relevant events are logged for compliance and forensics.
//! Events are stored in the security_events table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Security event record from database
#[derive(Debug, Clone, FromRow)]
pub struct SecurityEvent {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub event_type: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Security event for API responses (filtered view)
#[derive(Debug, Clone, Serialize)]
pub struct SecurityEventInfo {
    pub id: Uuid,
    pub event_type: String,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<SecurityEvent> for SecurityEventInfo {
    fn from(event: SecurityEvent) -> Self {
        Self {
            id: event.id,
            event_type: event.event_type,
            ip_address: event.ip_address,
            created_at: event.created_at,
        }
    }
}

/// Security events list response
#[derive(Debug, Serialize)]
pub struct SecurityEventsResponse {
    pub events: Vec<SecurityEventInfo>,
    pub total: usize,
}

/// Query parameters for security events
#[derive(Debug, Deserialize)]
pub struct SecurityEventsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub event_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_event_to_info() {
        let event = SecurityEvent {
            id: Uuid::new_v4(),
            user_id: Some(Uuid::new_v4()),
            event_type: "login_success".to_string(),
            ip_address: Some("192.168.1.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            metadata: None,
            created_at: Utc::now(),
        };

        let info: SecurityEventInfo = event.into();
        assert_eq!(info.event_type, "login_success");
    }
}
