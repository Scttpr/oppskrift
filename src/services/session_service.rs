//! Session management service
//!
//! Handles session creation, validation, revocation, and cleanup.
//! Sessions use secure random tokens with SHA-256 hashing for storage.

use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::net::IpAddr;
use thiserror::Error;
use uuid::Uuid;

/// Session token length in bytes (256 bits)
const TOKEN_BYTES: usize = 32;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found or expired")]
    NotFound,
    #[error("Session has expired")]
    Expired,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Session information for API responses
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: Uuid,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub last_activity: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

/// Session record from database
#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Session service for managing user sessions
#[derive(Clone)]
pub struct SessionService {
    pool: PgPool,
    expiry_days: u32,
}

impl SessionService {
    /// Create a new session service
    pub fn new(pool: PgPool, expiry_days: u32) -> Self {
        Self { pool, expiry_days }
    }

    /// Generate a secure random session token
    ///
    /// Returns (raw_token_hex, token_hash) where:
    /// - raw_token_hex: The token to send to the client (hex-encoded)
    /// - token_hash: The SHA-256 hash to store in the database
    fn generate_token() -> (String, String) {
        let mut token_bytes = [0u8; TOKEN_BYTES];
        rand::thread_rng().fill_bytes(&mut token_bytes);

        let raw_token = hex::encode(token_bytes);

        // Hash for storage
        let mut hasher = Sha256::new();
        hasher.update(token_bytes);
        let hash = hex::encode(hasher.finalize());

        (raw_token, hash)
    }

    /// Hash a token for lookup
    fn hash_token(token: &str) -> Result<String, SessionError> {
        let token_bytes = hex::decode(token).map_err(|_| SessionError::NotFound)?;

        let mut hasher = Sha256::new();
        hasher.update(&token_bytes);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Create a new session for a user
    ///
    /// Returns the session token to send to the client.
    /// The token should be stored in an HttpOnly, Secure, SameSite=Strict cookie.
    pub async fn create(
        &self,
        user_id: Uuid,
        ip_address: Option<IpAddr>,
        user_agent: Option<String>,
        device_info: Option<String>,
    ) -> Result<(Uuid, String, DateTime<Utc>), SessionError> {
        let (raw_token, token_hash) = Self::generate_token();
        let expires_at = Utc::now() + Duration::days(self.expiry_days as i64);
        let ip_str = ip_address.map(|ip| ip.to_string());

        let session_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO sessions (user_id, token_hash, device_info, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4::inet, $5, $6)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(&token_hash)
        .bind(&device_info)
        .bind(&ip_str)
        .bind(&user_agent)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok((session_id, raw_token, expires_at))
    }

    /// Validate a session token and return the user ID
    ///
    /// Also updates the last_activity timestamp.
    pub async fn validate(&self, token: &str) -> Result<(Uuid, Uuid), SessionError> {
        let token_hash = Self::hash_token(token)?;

        // Find and validate session
        let session: Option<(Uuid, Uuid, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT id, user_id, expires_at
            FROM sessions
            WHERE token_hash = $1
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let (id, user_id, expires_at) = session.ok_or(SessionError::NotFound)?;

        // Check expiration
        if expires_at < Utc::now() {
            // Clean up expired session
            let _ = self.revoke_by_id(id).await;
            return Err(SessionError::Expired);
        }

        // Update last activity
        sqlx::query("UPDATE sessions SET last_activity = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok((id, user_id))
    }

    /// Revoke a session by ID
    pub async fn revoke_by_id(&self, session_id: Uuid) -> Result<bool, SessionError> {
        let result = sqlx::query!("DELETE FROM sessions WHERE id = $1", session_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke a session by token
    pub async fn revoke_by_token(&self, token: &str) -> Result<bool, SessionError> {
        let token_hash = Self::hash_token(token)?;

        let result = sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", token_hash)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke all sessions for a user
    ///
    /// Returns the number of sessions revoked.
    pub async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<u64, SessionError> {
        let result = sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Revoke all sessions for a user except the current one
    ///
    /// Returns the number of sessions revoked.
    pub async fn revoke_others_for_user(
        &self,
        user_id: Uuid,
        current_session_id: Uuid,
    ) -> Result<u64, SessionError> {
        let result = sqlx::query!(
            "DELETE FROM sessions WHERE user_id = $1 AND id != $2",
            user_id,
            current_session_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// List all active sessions for a user
    pub async fn list_for_user(
        &self,
        user_id: Uuid,
        current_session_id: Option<Uuid>,
    ) -> Result<Vec<SessionInfo>, SessionError> {
        let sessions = sqlx::query!(
            r#"
            SELECT id, device_info, ip_address::text as ip_address, last_activity, created_at
            FROM sessions
            WHERE user_id = $1 AND expires_at > NOW()
            ORDER BY last_activity DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(sessions
            .into_iter()
            .map(|s| SessionInfo {
                id: s.id,
                device_info: s.device_info,
                ip_address: s.ip_address,
                last_activity: s.last_activity,
                created_at: s.created_at,
                is_current: current_session_id == Some(s.id),
            })
            .collect())
    }

    /// Clean up expired sessions
    ///
    /// Should be called periodically (e.g., daily) by a background job.
    /// Returns the number of sessions cleaned up.
    pub async fn cleanup_expired(&self) -> Result<u64, SessionError> {
        let result = sqlx::query!("DELETE FROM sessions WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;

        if result.rows_affected() > 0 {
            tracing::info!(
                count = result.rows_affected(),
                "Cleaned up expired sessions"
            );
        }

        Ok(result.rows_affected())
    }

    /// Get session count for a user
    pub async fn count_for_user(&self, user_id: Uuid) -> Result<i64, SessionError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM sessions WHERE user_id = $1 AND expires_at > NOW()",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let (token, hash) = SessionService::generate_token();

        // Token should be 64 hex chars (32 bytes)
        assert_eq!(token.len(), 64);
        // Hash should be 64 hex chars (SHA-256 = 32 bytes)
        assert_eq!(hash.len(), 64);

        // Verify hash matches
        let computed_hash = SessionService::hash_token(&token).unwrap();
        assert_eq!(hash, computed_hash);
    }

    #[test]
    fn test_hash_token_invalid() {
        // Invalid hex should return error
        assert!(SessionService::hash_token("not-hex").is_err());
    }
}
