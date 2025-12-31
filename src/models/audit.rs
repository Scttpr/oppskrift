//! Permission audit log model for security auditing
//!
//! Immutable log of all permission-related changes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

/// Event types for the permission audit log
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    PermissionGranted,
    PermissionRevoked,
    GroupCreated,
    GroupDeleted,
    MemberAdded,
    MemberRemoved,
    VisibilityChanged,
    AccessDenied,
}

impl AuditEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditEventType::PermissionGranted => "permission_granted",
            AuditEventType::PermissionRevoked => "permission_revoked",
            AuditEventType::GroupCreated => "group_created",
            AuditEventType::GroupDeleted => "group_deleted",
            AuditEventType::MemberAdded => "member_added",
            AuditEventType::MemberRemoved => "member_removed",
            AuditEventType::VisibilityChanged => "visibility_changed",
            AuditEventType::AccessDenied => "access_denied",
        }
    }
}

impl std::fmt::Display for AuditEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Permission audit log entry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PermissionAuditLog {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub actor_id: Option<Uuid>,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub subject_type: Option<String>,
    pub subject_id: Option<Uuid>,
    pub permission_level: Option<String>,
    pub details: JsonValue,
}

/// Request to create an audit log entry
#[derive(Debug, Clone)]
pub struct CreateAuditLog {
    pub event_type: AuditEventType,
    pub actor_id: Option<Uuid>,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub subject_type: Option<String>,
    pub subject_id: Option<Uuid>,
    pub permission_level: Option<String>,
    pub details: JsonValue,
}

impl CreateAuditLog {
    /// Create a new audit log entry for permission granted
    pub fn permission_granted(
        actor_id: Uuid,
        resource_type: &str,
        resource_id: Uuid,
        subject_type: &str,
        subject_id: Option<Uuid>,
        permission_level: &str,
    ) -> Self {
        Self {
            event_type: AuditEventType::PermissionGranted,
            actor_id: Some(actor_id),
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id),
            subject_type: Some(subject_type.to_string()),
            subject_id,
            permission_level: Some(permission_level.to_string()),
            details: serde_json::json!({}),
        }
    }

    /// Create a new audit log entry for permission revoked
    pub fn permission_revoked(
        actor_id: Uuid,
        resource_type: &str,
        resource_id: Uuid,
        subject_type: &str,
        subject_id: Option<Uuid>,
        permission_level: &str,
    ) -> Self {
        Self {
            event_type: AuditEventType::PermissionRevoked,
            actor_id: Some(actor_id),
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id),
            subject_type: Some(subject_type.to_string()),
            subject_id,
            permission_level: Some(permission_level.to_string()),
            details: serde_json::json!({}),
        }
    }

    /// Create a new audit log entry for group created
    pub fn group_created(actor_id: Uuid, group_id: Uuid, group_name: &str) -> Self {
        Self {
            event_type: AuditEventType::GroupCreated,
            actor_id: Some(actor_id),
            resource_type: Some("group".to_string()),
            resource_id: Some(group_id),
            subject_type: None,
            subject_id: None,
            permission_level: None,
            details: serde_json::json!({ "group_name": group_name }),
        }
    }

    /// Create a new audit log entry for group deleted
    pub fn group_deleted(actor_id: Uuid, group_id: Uuid, group_name: &str) -> Self {
        Self {
            event_type: AuditEventType::GroupDeleted,
            actor_id: Some(actor_id),
            resource_type: Some("group".to_string()),
            resource_id: Some(group_id),
            subject_type: None,
            subject_id: None,
            permission_level: None,
            details: serde_json::json!({ "group_name": group_name }),
        }
    }

    /// Create a new audit log entry for member added
    pub fn member_added(actor_id: Uuid, group_id: Uuid, user_id: Uuid) -> Self {
        Self {
            event_type: AuditEventType::MemberAdded,
            actor_id: Some(actor_id),
            resource_type: Some("group".to_string()),
            resource_id: Some(group_id),
            subject_type: Some("user".to_string()),
            subject_id: Some(user_id),
            permission_level: None,
            details: serde_json::json!({}),
        }
    }

    /// Create a new audit log entry for member removed
    pub fn member_removed(actor_id: Uuid, group_id: Uuid, user_id: Uuid) -> Self {
        Self {
            event_type: AuditEventType::MemberRemoved,
            actor_id: Some(actor_id),
            resource_type: Some("group".to_string()),
            resource_id: Some(group_id),
            subject_type: Some("user".to_string()),
            subject_id: Some(user_id),
            permission_level: None,
            details: serde_json::json!({}),
        }
    }

    /// Create a new audit log entry for visibility changed
    pub fn visibility_changed(
        actor_id: Uuid,
        resource_type: &str,
        resource_id: Uuid,
        old_visibility: &str,
        new_visibility: &str,
    ) -> Self {
        Self {
            event_type: AuditEventType::VisibilityChanged,
            actor_id: Some(actor_id),
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id),
            subject_type: None,
            subject_id: None,
            permission_level: None,
            details: serde_json::json!({
                "old_visibility": old_visibility,
                "new_visibility": new_visibility
            }),
        }
    }

    /// Create a new audit log entry for access denied
    pub fn access_denied(
        actor_id: Option<Uuid>,
        resource_type: &str,
        resource_id: Uuid,
        required_level: &str,
    ) -> Self {
        Self {
            event_type: AuditEventType::AccessDenied,
            actor_id,
            resource_type: Some(resource_type.to_string()),
            resource_id: Some(resource_id),
            subject_type: None,
            subject_id: None,
            permission_level: Some(required_level.to_string()),
            details: serde_json::json!({}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_type_as_str() {
        assert_eq!(
            AuditEventType::PermissionGranted.as_str(),
            "permission_granted"
        );
        assert_eq!(
            AuditEventType::PermissionRevoked.as_str(),
            "permission_revoked"
        );
        assert_eq!(AuditEventType::GroupCreated.as_str(), "group_created");
        assert_eq!(AuditEventType::GroupDeleted.as_str(), "group_deleted");
        assert_eq!(AuditEventType::MemberAdded.as_str(), "member_added");
        assert_eq!(AuditEventType::MemberRemoved.as_str(), "member_removed");
        assert_eq!(
            AuditEventType::VisibilityChanged.as_str(),
            "visibility_changed"
        );
        assert_eq!(AuditEventType::AccessDenied.as_str(), "access_denied");
    }

    #[test]
    fn test_audit_event_type_display() {
        assert_eq!(
            AuditEventType::PermissionGranted.to_string(),
            "permission_granted"
        );
    }

    #[test]
    fn test_create_audit_log_permission_granted() {
        let actor = Uuid::new_v4();
        let resource = Uuid::new_v4();
        let subject = Uuid::new_v4();

        let log = CreateAuditLog::permission_granted(
            actor,
            "recipe",
            resource,
            "user",
            Some(subject),
            "view",
        );

        assert_eq!(log.event_type, AuditEventType::PermissionGranted);
        assert_eq!(log.actor_id, Some(actor));
        assert_eq!(log.resource_type, Some("recipe".to_string()));
        assert_eq!(log.resource_id, Some(resource));
        assert_eq!(log.subject_id, Some(subject));
    }

    #[test]
    fn test_create_audit_log_visibility_changed() {
        let actor = Uuid::new_v4();
        let resource = Uuid::new_v4();

        let log =
            CreateAuditLog::visibility_changed(actor, "recipe", resource, "private", "public");

        assert_eq!(log.event_type, AuditEventType::VisibilityChanged);
        assert_eq!(log.details["old_visibility"], "private");
        assert_eq!(log.details["new_visibility"], "public");
    }

    #[test]
    fn test_create_audit_log_group_created() {
        let actor = Uuid::new_v4();
        let group = Uuid::new_v4();

        let log = CreateAuditLog::group_created(actor, group, "Family");

        assert_eq!(log.event_type, AuditEventType::GroupCreated);
        assert_eq!(log.details["group_name"], "Family");
    }
}
