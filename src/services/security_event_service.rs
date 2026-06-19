//! Security event service
//!
//! Provides methods for querying security events (audit log).
//! Security events are created by the AuditEvent system in core/audit.rs.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppError;
use crate::core::pagination::{paginate, PaginatedResponse, PaginationParams};

/// Security event record from the database
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SecurityEvent {
    pub id: Uuid,
    pub event_type: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Security event service for querying audit logs
pub struct SecurityEventService;

impl SecurityEventService {
    /// Count security events for a user
    pub async fn count_for_user(db: &PgPool, user_id: Uuid) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM security_events WHERE user_id = $1",
            user_id
        )
        .fetch_one(db)
        .await?
        .unwrap_or(0);

        Ok(count)
    }

    /// List security events for a user with pagination
    pub async fn list_for_user(
        db: &PgPool,
        user_id: Uuid,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<SecurityEvent>, AppError> {
        let offset = (page.saturating_sub(1)) * per_page;

        let events = sqlx::query_as!(
            SecurityEvent,
            r#"
            SELECT
                id,
                event_type::text as "event_type!",
                ip_address::text as ip_address,
                user_agent,
                metadata,
                created_at
            FROM security_events
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            per_page as i64,
            offset as i64,
        )
        .fetch_all(db)
        .await?;

        Ok(events)
    }

    /// List security events for a user with standard pagination params
    pub async fn list_paginated(
        db: &PgPool,
        user_id: Uuid,
        params: &PaginationParams,
    ) -> Result<PaginatedResponse<SecurityEvent>, AppError> {
        paginate(
            params,
            |limit, offset| {
                sqlx::query_as!(
                    SecurityEvent,
                    r#"
                    SELECT
                        id,
                        event_type::text as "event_type!",
                        ip_address::text as ip_address,
                        user_agent,
                        metadata,
                        created_at
                    FROM security_events
                    WHERE user_id = $1
                    ORDER BY created_at DESC
                    LIMIT $2 OFFSET $3
                    "#,
                    user_id,
                    limit,
                    offset,
                )
                .fetch_all(db)
            },
            || {
                sqlx::query_scalar!(
                    r#"SELECT COUNT(*) as "count!" FROM security_events WHERE user_id = $1"#,
                    user_id
                )
                .fetch_one(db)
            },
        )
        .await
    }

    /// Get the most recent events for a user (simple limit query)
    pub async fn get_recent(
        db: &PgPool,
        user_id: Uuid,
        limit: i64,
    ) -> Result<Vec<SecurityEvent>, AppError> {
        let events = sqlx::query_as::<_, SecurityEvent>(
            r#"
            SELECT id, event_type::text as event_type, ip_address::text as ip_address, user_agent, metadata, created_at
            FROM security_events
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(db)
        .await?;

        Ok(events)
    }

    /// Get the last export event time for a user (for rate limiting)
    pub async fn get_last_export_time(
        db: &PgPool,
        user_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        let last_export = sqlx::query_scalar!(
            r#"
            SELECT created_at FROM security_events
            WHERE user_id = $1 AND event_type::text = 'account_export'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(db)
        .await?;

        Ok(last_export)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_event_struct() {
        // Verify the struct can be created
        let event = SecurityEvent {
            id: Uuid::new_v4(),
            event_type: "login_success".to_string(),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("Test Agent".to_string()),
            metadata: None,
            created_at: Utc::now(),
        };
        assert_eq!(event.event_type, "login_success");
    }
}
