//! Background cleanup jobs
//!
//! Scheduled jobs for:
//! - Expired session cleanup
//! - Expired token cleanup (password reset, email confirmation)
//! - Account deletion execution after grace period
//!
//! These jobs should be run periodically (e.g., daily via cron or tokio-cron).
//! The CleanupService is designed to be called by an external scheduler,
//! not directly from the web application.

#![allow(dead_code)] // Module is called by external scheduler, not from app

use chrono::{Duration, Utc};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::core::audit::AuditEvent;

/// Cleanup job errors
#[derive(Debug, Error)]
pub enum CleanupError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Cleanup job results
#[derive(Debug, Default)]
pub struct CleanupResult {
    pub sessions_removed: u64,
    pub password_reset_tokens_removed: u64,
    pub email_confirmation_tokens_removed: u64,
    pub two_factor_pending_tokens_removed: u64,
    pub accounts_deleted: u64,
}

/// Cleanup service for background maintenance tasks
pub struct CleanupService {
    pool: PgPool,
}

impl CleanupService {
    /// Create a new cleanup service
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run all cleanup tasks (T078, T082)
    ///
    /// This should be called periodically (e.g., daily).
    /// Returns a summary of cleaned up items.
    pub async fn run_all(&self) -> Result<CleanupResult, CleanupError> {
        let mut result = CleanupResult::default();

        // Clean up expired sessions
        result.sessions_removed = self.cleanup_expired_sessions().await?;

        // Clean up expired password reset tokens
        result.password_reset_tokens_removed = self.cleanup_expired_password_reset_tokens().await?;

        // Clean up expired email confirmation tokens
        result.email_confirmation_tokens_removed =
            self.cleanup_expired_email_confirmation_tokens().await?;

        // Clean up expired 2FA pending tokens
        result.two_factor_pending_tokens_removed =
            self.cleanup_expired_2fa_pending_tokens().await?;

        // Execute pending account deletions
        result.accounts_deleted = self.execute_pending_deletions().await?;

        tracing::info!(
            sessions = result.sessions_removed,
            password_tokens = result.password_reset_tokens_removed,
            email_tokens = result.email_confirmation_tokens_removed,
            two_factor_tokens = result.two_factor_pending_tokens_removed,
            accounts = result.accounts_deleted,
            "Cleanup job completed"
        );

        Ok(result)
    }

    /// Clean up expired sessions (T082)
    ///
    /// Removes sessions that have passed their expires_at timestamp.
    pub async fn cleanup_expired_sessions(&self) -> Result<u64, CleanupError> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;

        let removed = result.rows_affected();
        if removed > 0 {
            tracing::info!(count = removed, "Cleaned up expired sessions");
        }

        Ok(removed)
    }

    /// Clean up expired password reset tokens
    ///
    /// Removes tokens that have expired or been used more than 24 hours ago.
    pub async fn cleanup_expired_password_reset_tokens(&self) -> Result<u64, CleanupError> {
        let result = sqlx::query(
            r#"
            DELETE FROM password_reset_tokens
            WHERE expires_at < NOW()
               OR (used_at IS NOT NULL AND used_at < NOW() - INTERVAL '24 hours')
            "#,
        )
        .execute(&self.pool)
        .await?;

        let removed = result.rows_affected();
        if removed > 0 {
            tracing::info!(count = removed, "Cleaned up password reset tokens");
        }

        Ok(removed)
    }

    /// Clean up expired email confirmation tokens
    ///
    /// Removes tokens that have expired.
    pub async fn cleanup_expired_email_confirmation_tokens(&self) -> Result<u64, CleanupError> {
        let result = sqlx::query("DELETE FROM email_confirmation_tokens WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;

        let removed = result.rows_affected();
        if removed > 0 {
            tracing::info!(count = removed, "Cleaned up email confirmation tokens");
        }

        Ok(removed)
    }

    /// Clean up expired 2FA pending tokens
    ///
    /// Removes tokens that have expired (5-minute lifetime).
    pub async fn cleanup_expired_2fa_pending_tokens(&self) -> Result<u64, CleanupError> {
        let result = sqlx::query("DELETE FROM two_factor_pending_tokens WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;

        let removed = result.rows_affected();
        if removed > 0 {
            tracing::info!(count = removed, "Cleaned up 2FA pending tokens");
        }

        Ok(removed)
    }

    /// Execute pending account deletions after grace period (T075)
    ///
    /// Finds accounts with deletion_requested_at older than 7 days
    /// and executes the deletion.
    pub async fn execute_pending_deletions(&self) -> Result<u64, CleanupError> {
        // Find accounts ready for deletion (grace period passed)
        let grace_period_days = 7i64;
        let cutoff = Utc::now() - Duration::days(grace_period_days);

        let accounts: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
            FROM users
            WHERE deletion_requested_at IS NOT NULL
              AND deletion_requested_at < $1
            "#,
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        let mut deleted = 0u64;
        for user_id in accounts {
            match self.execute_deletion(user_id).await {
                Ok(()) => {
                    deleted += 1;
                    tracing::info!(user_id = %user_id, "Account deleted after grace period");
                }
                Err(e) => {
                    tracing::error!(user_id = %user_id, error = %e, "Failed to delete account");
                }
            }
        }

        Ok(deleted)
    }

    /// Execute a single account deletion (T075)
    ///
    /// Hard deletes all user data. Recipes are anonymized (author_id set to NULL).
    /// This is GDPR compliant - all PII is removed.
    pub async fn execute_deletion(&self, user_id: Uuid) -> Result<(), CleanupError> {
        // Log the deletion event before we delete the user
        AuditEvent::new("auth.account.delete.execute")
            .with_user(user_id)
            .warn()
            .persist(&self.pool)
            .await;

        // Start transaction for atomic deletion
        let mut tx = self.pool.begin().await?;

        // 1. Delete sessions
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 2. Delete email confirmation tokens
        sqlx::query("DELETE FROM email_confirmation_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 3. Delete password reset tokens
        sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 4. Delete 2FA pending tokens
        sqlx::query("DELETE FROM two_factor_pending_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 5. Delete recovery codes
        sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 6. Delete follows (both directions)
        sqlx::query("DELETE FROM follows WHERE follower_id = $1 OR following_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 7. Delete saved recipes
        sqlx::query("DELETE FROM saved_recipes WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 8. Delete recipe book entries and books
        sqlx::query(
            r#"
            DELETE FROM book_recipe_entries
            WHERE book_id IN (SELECT id FROM recipe_books WHERE user_id = $1)
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM recipe_books WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 9. Delete activities
        sqlx::query("DELETE FROM activities WHERE actor_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 10. Anonymize recipes (set author_id to NULL, preserving content)
        sqlx::query("UPDATE recipes SET author_id = NULL WHERE author_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 11. Delete security events (optional - could keep for audit)
        // For GDPR compliance, we delete them
        sqlx::query("DELETE FROM security_events WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // 12. Finally, delete the user
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_result_default() {
        let result = CleanupResult::default();

        assert_eq!(result.sessions_removed, 0);
        assert_eq!(result.password_reset_tokens_removed, 0);
        assert_eq!(result.email_confirmation_tokens_removed, 0);
        assert_eq!(result.two_factor_pending_tokens_removed, 0);
        assert_eq!(result.accounts_deleted, 0);
    }

    #[test]
    fn test_grace_period_calculation() {
        let grace_period_days = 7i64;
        let now = Utc::now();
        let cutoff = now - Duration::days(grace_period_days);

        // Accounts requested before cutoff should be deleted
        let old_request = now - Duration::days(8);
        assert!(old_request < cutoff);

        // Accounts requested after cutoff should NOT be deleted
        let recent_request = now - Duration::days(3);
        assert!(recent_request > cutoff);
    }
}
