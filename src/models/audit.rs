//! Permission audit log model for security auditing
//!
//! Immutable log of all permission-related changes. Entries are written through
//! the unified [`crate::core::audit::AuditEvent`] interface (permission/access
//! events route here); this struct is the read model for that table.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

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
