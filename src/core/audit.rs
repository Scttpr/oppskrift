//! Audit logging module
//! Provides structured audit events for security-sensitive actions

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::request_id::RequestContext;

/// Structured audit event for security logging
#[derive(Debug, Serialize)]
pub struct AuditEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event: String,
    pub level: String,
    /// Unique ID for this specific event
    pub trace_id: Uuid,
    /// Shared ID for all events in a single HTTP request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<Uuid>,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Uuid>,
    /// Subject a permission is granted to (permission/access events only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<Uuid>,
    /// Permission level in play (permission/access events only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_level: Option<String>,
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
            request_id: None,
            service: "oppskrift".to_string(),
            user_id: None,
            session_id: None,
            ip: None,
            target_type: None,
            target_id: None,
            subject_type: None,
            subject_id: None,
            permission_level: None,
            metadata: None,
        }
    }

    /// Add request ID to correlate events from the same HTTP request
    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Add optional request ID to the event
    pub fn maybe_request_id(mut self, request_id: Option<Uuid>) -> Self {
        self.request_id = request_id;
        self
    }

    /// Add request context (request_id, ip, session_id) to the event
    pub fn with_context(mut self, ctx: &RequestContext) -> Self {
        self.request_id = ctx.request_id;
        self.ip = ctx.ip.map(|ip| ip.to_string());
        self.session_id = ctx.session_id;
        self
    }

    /// Add user ID to the event
    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Add optional user ID to the event
    pub fn maybe_user(mut self, user_id: Option<Uuid>) -> Self {
        self.user_id = user_id;
        self
    }

    /// Add the subject a permission applies to (permission/access events)
    pub fn with_subject(mut self, subject_type: &str, subject_id: Option<Uuid>) -> Self {
        self.subject_type = Some(subject_type.to_string());
        self.subject_id = subject_id;
        self
    }

    /// Add the permission level in play (permission/access events)
    pub fn with_level(mut self, level: &str) -> Self {
        self.permission_level = Some(level.to_string());
        self
    }

    /// Add session ID to the event
    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Add optional session ID to the event
    pub fn maybe_session(mut self, session_id: Option<Uuid>) -> Self {
        self.session_id = session_id;
        self
    }

    /// Add IP address to the event
    pub fn with_ip(mut self, ip: impl std::fmt::Display) -> Self {
        self.ip = Some(ip.to_string());
        self
    }

    /// Add optional IP address to the event
    pub fn maybe_ip(mut self, ip: Option<impl std::fmt::Display>) -> Self {
        self.ip = ip.map(|i| i.to_string());
        self
    }

    /// Add target resource to the event
    pub fn with_target(mut self, target_type: &str, target_id: Uuid) -> Self {
        self.target_type = Some(target_type.to_string());
        self.target_id = Some(target_id);
        self
    }

    /// Add metadata to the event
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        let metadata = self
            .metadata
            .take()
            .unwrap_or_else(|| serde_json::json!({}));
        if let serde_json::Value::Object(mut map) = metadata {
            map.insert(
                key.to_string(),
                serde_json::Value::String(value.to_string()),
            );
            self.metadata = Some(serde_json::Value::Object(map));
        }
        self
    }

    /// Set level to warn
    pub fn warn(mut self) -> Self {
        self.level = "warn".to_string();
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

    /// Log the event and persist to database for user-visible security history
    ///
    /// This should be used for auth-related events that users should be able to see.
    /// Falls back to just logging if persistence fails.
    pub async fn persist(self, pool: &PgPool) {
        // Log to tracing first
        let json = serde_json::to_string(&self).unwrap_or_default();
        match self.level.as_str() {
            "warn" => tracing::warn!(audit = %json, "audit event"),
            "error" => tracing::error!(audit = %json, "audit event"),
            _ => tracing::info!(audit = %json, "audit event"),
        }

        // Permission/access events cross into the relational audit trail
        // (immutable, partitioned) rather than the self-centric security log.
        let permission_event_type = match self.event.as_str() {
            "permission.granted" => Some("permission_granted"),
            "permission.revoked" => Some("permission_revoked"),
            "access.denied" => Some("access_denied"),
            _ => None,
        };
        if let Some(event_type) = permission_event_type {
            let details = self
                .metadata
                .clone()
                .unwrap_or_else(|| serde_json::json!({}));
            let result = sqlx::query(
                r#"
                INSERT INTO permission_audit_log (
                    event_type, actor_id, resource_type, resource_id,
                    subject_type, subject_id, permission_level, details
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(event_type)
            .bind(self.user_id)
            .bind(&self.target_type)
            .bind(self.target_id)
            .bind(&self.subject_type)
            .bind(self.subject_id)
            .bind(&self.permission_level)
            .bind(&details)
            .execute(pool)
            .await;
            if let Err(e) = result {
                tracing::error!(
                    error = %e,
                    event = %self.event,
                    trace_id = %self.trace_id,
                    "Failed to persist permission audit event"
                );
            }
            return;
        }

        // Map event name to DB enum (e.g., "auth.login.success" -> "login_success")
        let db_event_type = match self.event.as_str() {
            "auth.register.success" => "register_success",
            "auth.register.failure" => "register_failure",
            "auth.login.success" => "login_success",
            "auth.login.failure" => "login_failure",
            "auth.login.locked" => "login_locked",
            "auth.logout" => "logout",
            "auth.password.reset.request" => "password_reset_request",
            "auth.password.reset.complete" => "password_reset_complete",
            "auth.password.change" | "auth.password.change.failure" => "password_change",
            "auth.email.change" | "auth.email.change.request" => "email_change",
            "auth.email.confirmed" => "email_confirmed",
            "auth.2fa.enable" => "totp_enable",
            "auth.2fa.disable" => "totp_disable",
            "auth.2fa.failure" | "auth.2fa.recovery.used" | "auth.2fa.recovery.regenerated" => {
                "recovery_code_used"
            }
            "auth.session.revoke" => "session_revoke",
            "auth.session.revoke.all" => "session_revoke_all",
            "auth.account.delete.request" => "account_delete_request",
            "auth.account.delete.cancel" => "account_delete_cancel",
            "auth.account.delete.execute" => "account_delete_execute",
            _ => {
                // Non-auth events don't get persisted to security_events table
                return;
            }
        };

        // Merge session_id into metadata for DB storage
        let metadata = {
            let mut meta = self
                .metadata
                .clone()
                .unwrap_or_else(|| serde_json::json!({}));
            if let Some(sid) = self.session_id {
                if let serde_json::Value::Object(ref mut map) = meta {
                    map.insert(
                        "session_id".to_string(),
                        serde_json::Value::String(sid.to_string()),
                    );
                }
            }
            if meta == serde_json::json!({}) {
                None
            } else {
                Some(meta)
            }
        };

        // Persist to database
        let result = sqlx::query(
            r#"
            INSERT INTO security_events (user_id, event_type, ip_address, metadata)
            VALUES ($1, $2::security_event_type, $3::inet, $4)
            "#,
        )
        .bind(self.user_id)
        .bind(db_event_type)
        .bind(&self.ip)
        .bind(&metadata)
        .execute(pool)
        .await;

        match result {
            Ok(_) => {
                tracing::debug!(
                    event = %self.event,
                    trace_id = %self.trace_id,
                    request_id = ?self.request_id,
                    user_id = ?self.user_id,
                    session_id = ?self.session_id,
                    ip = ?self.ip,
                    "Audit event persisted"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    event = %self.event,
                    trace_id = %self.trace_id,
                    request_id = ?self.request_id,
                    user_id = ?self.user_id,
                    session_id = ?self.session_id,
                    ip = ?self.ip,
                    "Failed to persist audit event"
                );
            }
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
