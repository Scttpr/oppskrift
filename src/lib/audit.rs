//! Audit logging module
//! Provides structured audit events for security-sensitive actions

use serde::Serialize;
use uuid::Uuid;

/// Structured audit event for security logging
#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event: String,
    pub level: String,
    pub trace_id: Uuid,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl AuditEvent {
    /// Create a new audit event with the given event name
    pub fn new(event: &str) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            event: event.to_string(),
            level: "info".to_string(),
            trace_id: Uuid::new_v4(),
            service: "oppskrift".to_string(),
            user_id: None,
            ip: None,
            target_type: None,
            target_id: None,
            metadata: None,
        }
    }

    /// Add user ID to the event
    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Add IP address to the event
    pub fn with_ip(mut self, ip: &str) -> Self {
        self.ip = Some(ip.to_string());
        self
    }

    /// Add target resource to the event
    pub fn with_target(mut self, target_type: &str, target_id: Uuid) -> Self {
        self.target_type = Some(target_type.to_string());
        self.target_id = Some(target_id);
        self
    }

    /// Add metadata to the event
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set level to warn
    pub fn warn(mut self) -> Self {
        self.level = "warn".to_string();
        self
    }

    /// Set level to error
    pub fn error(mut self) -> Self {
        self.level = "error".to_string();
        self
    }

    /// Log the event using tracing
    pub fn log(self) {
        let json = serde_json::to_string(&self).unwrap_or_default();
        match self.level.as_str() {
            "warn" => tracing::warn!(audit = %json, "audit event"),
            "error" => tracing::error!(audit = %json, "audit event"),
            _ => tracing::info!(audit = %json, "audit event"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new("auth.login.success");
        assert_eq!(event.event, "auth.login.success");
        assert_eq!(event.level, "info");
        assert_eq!(event.service, "oppskrift");
    }

    #[test]
    fn test_audit_event_with_user() {
        let user_id = Uuid::new_v4();
        let event = AuditEvent::new("user.create").with_user(user_id);
        assert_eq!(event.user_id, Some(user_id));
    }

    #[test]
    fn test_audit_event_warn_level() {
        let event = AuditEvent::new("auth.login.failure").warn();
        assert_eq!(event.level, "warn");
    }
}
