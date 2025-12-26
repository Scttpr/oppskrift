//! Data cleanup jobs
//!
//! Handles periodic cleanup of old data according to retention policies.

use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::{interval, Duration};

use crate::lib::audit::AuditEvent;

/// Configuration for data retention periods
#[derive(Debug, Clone)]
pub struct RetentionConfig {
    /// Days to keep audit logs
    pub audit_log_days: i64,
    /// Days to keep expired sessions
    pub session_days: i64,
    /// Days to keep deleted content references
    pub deleted_content_days: i64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            audit_log_days: 90,
            session_days: 30,
            deleted_content_days: 30,
        }
    }
}

/// Worker that runs periodic cleanup tasks
pub struct CleanupWorker {
    pool: Arc<PgPool>,
    config: RetentionConfig,
}

impl CleanupWorker {
    /// Create a new cleanup worker
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self {
            pool,
            config: RetentionConfig::default(),
        }
    }

    /// Create with custom retention config
    pub fn with_config(pool: Arc<PgPool>, config: RetentionConfig) -> Self {
        Self { pool, config }
    }

    /// Run the cleanup worker with periodic execution
    pub async fn run(self) {
        tracing::info!("Cleanup worker started");

        // Run cleanup every hour
        let mut interval = interval(Duration::from_secs(3600));

        loop {
            interval.tick().await;
            self.run_cleanup().await;
        }
    }

    /// Run all cleanup tasks
    pub async fn run_cleanup(&self) {
        tracing::info!("Starting scheduled cleanup");

        let mut total_deleted = 0u64;

        // Cleanup audit logs
        match self.cleanup_audit_logs().await {
            Ok(count) => {
                total_deleted += count;
                if count > 0 {
                    tracing::info!(count, "Cleaned up old audit logs");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to cleanup audit logs");
            }
        }

        // Cleanup expired sessions
        match self.cleanup_expired_sessions().await {
            Ok(count) => {
                total_deleted += count;
                if count > 0 {
                    tracing::info!(count, "Cleaned up expired sessions");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to cleanup sessions");
            }
        }

        // Cleanup orphaned federation deliveries
        match self.cleanup_failed_deliveries().await {
            Ok(count) => {
                total_deleted += count;
                if count > 0 {
                    tracing::info!(count, "Cleaned up failed deliveries");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to cleanup deliveries");
            }
        }

        // Log completion
        AuditEvent::new("system.cleanup.completed")
            .with_metadata("total_deleted", &total_deleted.to_string())
            .log();

        tracing::info!(total_deleted, "Cleanup completed");
    }

    /// Delete audit logs older than retention period
    async fn cleanup_audit_logs(&self) -> Result<u64, sqlx::Error> {
        let query = format!(
            "DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '{} days'",
            self.config.audit_log_days
        );

        match sqlx::query(&query).execute(self.pool.as_ref()).await {
            Ok(r) => Ok(r.rows_affected()),
            Err(sqlx::Error::Database(ref e)) if e.message().contains("does not exist") => {
                // Table doesn't exist yet, skip
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }

    /// Delete expired sessions
    async fn cleanup_expired_sessions(&self) -> Result<u64, sqlx::Error> {
        let query = format!(
            "DELETE FROM sessions WHERE expires_at < NOW() OR created_at < NOW() - INTERVAL '{} days'",
            self.config.session_days
        );

        match sqlx::query(&query).execute(self.pool.as_ref()).await {
            Ok(r) => Ok(r.rows_affected()),
            Err(sqlx::Error::Database(ref e)) if e.message().contains("does not exist") => {
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }

    /// Delete old failed federation deliveries
    async fn cleanup_failed_deliveries(&self) -> Result<u64, sqlx::Error> {
        let query = format!(
            "DELETE FROM federation_deliveries WHERE status = 'failed' AND created_at < NOW() - INTERVAL '{} days'",
            self.config.deleted_content_days
        );

        match sqlx::query(&query).execute(self.pool.as_ref()).await {
            Ok(r) => Ok(r.rows_affected()),
            Err(sqlx::Error::Database(ref e)) if e.message().contains("does not exist") => {
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_retention_config() {
        let config = RetentionConfig::default();
        assert_eq!(config.audit_log_days, 90);
        assert_eq!(config.session_days, 30);
        assert_eq!(config.deleted_content_days, 30);
    }
}
