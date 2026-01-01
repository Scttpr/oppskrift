//! Session models for authentication

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// Format a datetime as a human-readable relative time string
pub fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff.num_seconds() < 60 {
        "just now".to_string()
    } else if diff.num_minutes() < 60 {
        let mins = diff.num_minutes();
        if mins == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", mins)
        }
    } else if diff.num_hours() < 24 {
        let hours = diff.num_hours();
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else if diff.num_days() < 30 {
        let days = diff.num_days();
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
    } else {
        dt.format("%Y-%m-%d").to_string()
    }
}

/// Session view for template display with formatted fields
#[derive(Debug, Clone, Serialize)]
pub struct SessionItemView {
    pub id: Uuid,
    pub device_info: String,
    pub ip_address: String,
    pub last_activity: String,
    pub created_at: String,
    pub is_current: bool,
}

impl SessionItemView {
    /// Create a view from SessionInfo with human-readable formatting
    pub fn from_session_info(info: &SessionInfo) -> Self {
        Self {
            id: info.id,
            device_info: info
                .device_info
                .clone()
                .unwrap_or_else(|| "Unknown device".to_string()),
            ip_address: info
                .ip_address
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            last_activity: format_relative_time(info.last_activity),
            created_at: info.created_at.format("%Y-%m-%d %H:%M UTC").to_string(),
            is_current: info.is_current,
        }
    }
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
