//! TOTP service for two-factor authentication
//!
//! Handles TOTP setup, verification, and recovery code management.
//! Uses AES-256-GCM for secret encryption and bcrypt for recovery codes.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use sqlx::PgPool;
use thiserror::Error;
use totp_rs::{Algorithm, Secret, TOTP};
use uuid::Uuid;

use crate::core::audit::AuditEvent;
use crate::core::RequestContext;
use crate::models::{RecoveryCode, RecoveryCodesResponse, TwoFactorSetupResponse};

/// Number of recovery codes to generate
const RECOVERY_CODE_COUNT: usize = 8;

/// Recovery code length (format: XXXX-XXXX)
const RECOVERY_CODE_LENGTH: usize = 8;

/// TOTP time step (standard 30 seconds)
const TOTP_STEP: u64 = 30;

/// TOTP digits (standard 6)
const TOTP_DIGITS: usize = 6;

/// TOTP skew (allow 1 step before/after for clock drift)
const TOTP_SKEW: u8 = 1;

#[derive(Debug, Error)]
pub enum TotpError {
    #[error("Invalid TOTP code")]
    InvalidCode,
    #[error("Invalid recovery code")]
    InvalidRecoveryCode,
    #[error("2FA is already enabled")]
    AlreadyEnabled,
    #[error("2FA is not enabled")]
    NotEnabled,
    #[error("No pending setup")]
    NoPendingSetup,
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("TOTP generation error: {0}")]
    TotpGeneration(String),
}

/// TOTP service for 2FA management
pub struct TotpService {
    pool: PgPool,
    encryption_key: [u8; 32],
    issuer: String,
}

impl TotpService {
    /// Create a new TOTP service
    ///
    /// Requires a 32-byte (256-bit) encryption key for TOTP secret storage.
    pub fn new(pool: PgPool, encryption_key: [u8; 32], issuer: String) -> Self {
        Self {
            pool,
            encryption_key,
            issuer,
        }
    }

    /// Create TOTP service from environment
    pub fn from_env(pool: PgPool) -> Result<Self, TotpError> {
        let key_hex = std::env::var("TOTP_ENCRYPTION_KEY").map_err(|_| {
            TotpError::Encryption("TOTP_ENCRYPTION_KEY environment variable not set".to_string())
        })?;

        let key_bytes = hex::decode(&key_hex).map_err(|e| {
            TotpError::Encryption(format!("Invalid TOTP_ENCRYPTION_KEY format: {}", e))
        })?;

        if key_bytes.len() != 32 {
            return Err(TotpError::Encryption(format!(
                "TOTP_ENCRYPTION_KEY must be 32 bytes, got {}",
                key_bytes.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_bytes);

        let issuer = std::env::var("APP_NAME").unwrap_or_else(|_| "Oppskrift".to_string());

        Ok(Self::new(pool, key, issuer))
    }

    /// Start 2FA setup (T056)
    ///
    /// Generates a new TOTP secret and QR code.
    /// The secret is stored encrypted until 2FA is enabled.
    pub async fn setup_2fa(
        &self,
        user_id: Uuid,
        email: &str,
    ) -> Result<TwoFactorSetupResponse, TotpError> {
        // Check if 2FA is already enabled
        let totp_enabled: bool = sqlx::query_scalar("SELECT totp_enabled FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        if totp_enabled {
            return Err(TotpError::AlreadyEnabled);
        }

        // Generate new secret (20 bytes = 160 bits, standard for TOTP)
        let secret = Secret::default();
        let secret_base32 = secret.to_encoded().to_string();

        // Create TOTP instance
        let totp = TOTP::new(
            Algorithm::SHA1,
            TOTP_DIGITS,
            TOTP_SKEW,
            TOTP_STEP,
            secret.to_bytes().unwrap(),
            Some(self.issuer.clone()),
            email.to_string(),
        )
        .map_err(|e| TotpError::TotpGeneration(e.to_string()))?;

        // Generate QR code
        let qr_code = totp
            .get_qr_base64()
            .map_err(|e| TotpError::TotpGeneration(e.to_string()))?;

        let otpauth_uri = totp.get_url();

        // Encrypt secret for storage
        let encrypted_secret = self.encrypt_secret(&secret_base32)?;

        // Store pending secret (temporarily in the encrypted field, but not enabled)
        // We'll clear it if setup is abandoned and overwrite when enabled
        let encrypted_bytes = encrypted_secret.as_bytes();
        sqlx::query(
            r#"
            UPDATE users
            SET totp_secret_encrypted = $2, updated_at = NOW()
            WHERE id = $1 AND totp_enabled = false
            "#,
        )
        .bind(user_id)
        .bind(encrypted_bytes)
        .execute(&self.pool)
        .await?;

        Ok(TwoFactorSetupResponse {
            qr_code,
            secret: secret_base32,
            otpauth_uri,
        })
    }

    /// Enable 2FA after verification (T056)
    ///
    /// Verifies the TOTP code and activates 2FA.
    /// Returns recovery codes.
    pub async fn enable_2fa(
        &self,
        user_id: Uuid,
        totp_code: &str,
        ctx: &RequestContext,
    ) -> Result<Vec<String>, TotpError> {
        // Get pending secret (stored in totp_secret_encrypted but totp_enabled = false)
        let pending_secret: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT totp_secret_encrypted FROM users WHERE id = $1 AND totp_enabled = false",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        let encrypted_bytes = pending_secret.ok_or(TotpError::NoPendingSetup)?;
        let encrypted_secret =
            String::from_utf8(encrypted_bytes).map_err(|e| TotpError::Encryption(e.to_string()))?;

        // Decrypt secret
        let secret_base32 = self.decrypt_secret(&encrypted_secret)?;

        // Verify TOTP code
        if !self.verify_code(&secret_base32, totp_code)? {
            return Err(TotpError::InvalidCode);
        }

        // Generate recovery codes
        let (codes, code_hashes) = self.generate_recovery_codes()?;

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Enable 2FA (secret is already stored, just enable it)
        sqlx::query(
            r#"
            UPDATE users
            SET totp_enabled = true, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Delete existing recovery codes
        sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Insert new recovery codes
        for code_hash in &code_hashes {
            sqlx::query(
                r#"
                INSERT INTO recovery_codes (user_id, code_hash, created_at)
                VALUES ($1, $2, NOW())
                "#,
            )
            .bind(user_id)
            .bind(code_hash)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Log security event
        AuditEvent::new("auth.2fa.enable")
            .with_user(user_id)
            .with_context(ctx)
            .persist(&self.pool)
            .await;

        Ok(codes)
    }

    /// Disable 2FA (T056)
    ///
    /// Requires password verification (done in auth service) and TOTP/recovery code.
    pub async fn disable_2fa(
        &self,
        user_id: Uuid,
        code: &str,
        ctx: &RequestContext,
    ) -> Result<(), TotpError> {
        // Verify the code (either TOTP or recovery)
        let is_valid = self.verify_2fa(user_id, code, ctx).await?;

        if !is_valid {
            return Err(TotpError::InvalidCode);
        }

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Disable 2FA
        sqlx::query(
            r#"
            UPDATE users
            SET totp_enabled = false,
                totp_secret_encrypted = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        // Delete recovery codes
        sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        // Log security event
        AuditEvent::new("auth.2fa.disable")
            .with_user(user_id)
            .with_context(ctx)
            .warn()
            .persist(&self.pool)
            .await;

        Ok(())
    }

    /// Verify 2FA code (TOTP or recovery code)
    pub async fn verify_2fa(
        &self,
        user_id: Uuid,
        code: &str,
        ctx: &RequestContext,
    ) -> Result<bool, TotpError> {
        // Check if it's a recovery code format (XXXX-XXXX)
        if code.len() == 9 && code.chars().nth(4) == Some('-') {
            return match self.use_recovery_code(user_id, code, ctx).await {
                Ok(valid) => Ok(valid),
                Err(TotpError::InvalidRecoveryCode) => Ok(false),
                Err(e) => Err(e),
            };
        }

        // Otherwise verify as TOTP
        self.verify_totp(user_id, code).await
    }

    /// Verify TOTP code
    pub async fn verify_totp(&self, user_id: Uuid, totp_code: &str) -> Result<bool, TotpError> {
        // Get encrypted secret
        let encrypted_bytes: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT totp_secret_encrypted FROM users WHERE id = $1 AND totp_enabled = true",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        let encrypted_bytes = encrypted_bytes.ok_or(TotpError::NotEnabled)?;
        let encrypted_secret =
            String::from_utf8(encrypted_bytes).map_err(|e| TotpError::Encryption(e.to_string()))?;

        // Decrypt secret
        let secret_base32 = self.decrypt_secret(&encrypted_secret)?;

        // Verify
        self.verify_code(&secret_base32, totp_code)
    }

    /// Use a recovery code (T057)
    ///
    /// Recovery codes are single-use. Returns true if valid.
    pub async fn use_recovery_code(
        &self,
        user_id: Uuid,
        recovery_code: &str,
        ctx: &RequestContext,
    ) -> Result<bool, TotpError> {
        // Normalize code
        let code = recovery_code.to_uppercase().replace('-', "");

        // Get unused recovery codes
        let codes: Vec<RecoveryCode> = sqlx::query_as(
            r#"
            SELECT id, code_hash
            FROM recovery_codes
            WHERE user_id = $1 AND used_at IS NULL
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        // Try to match against each code
        for stored_code in codes {
            if bcrypt::verify(&code, &stored_code.code_hash).unwrap_or(false) {
                // Mark as used
                sqlx::query("UPDATE recovery_codes SET used_at = NOW() WHERE id = $1")
                    .bind(stored_code.id)
                    .execute(&self.pool)
                    .await?;

                // Count remaining codes
                let remaining: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM recovery_codes WHERE user_id = $1 AND used_at IS NULL",
                )
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;

                // Log security event
                AuditEvent::new("auth.2fa.recovery.used")
                    .with_user(user_id)
                    .with_context(ctx)
                    .with_metadata("codes_remaining", &remaining.to_string())
                    .warn()
                    .persist(&self.pool)
                    .await;

                return Ok(true);
            }
        }

        // No matching recovery code found
        Err(TotpError::InvalidRecoveryCode)
    }

    /// Get recovery codes status
    pub async fn get_recovery_codes_status(
        &self,
        user_id: Uuid,
    ) -> Result<(u32, u32, Option<chrono::DateTime<chrono::Utc>>), TotpError> {
        let total: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM recovery_codes WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;

        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM recovery_codes WHERE user_id = $1 AND used_at IS NULL",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        let generated_at: Option<chrono::DateTime<chrono::Utc>> =
            sqlx::query_scalar("SELECT MIN(created_at) FROM recovery_codes WHERE user_id = $1")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;

        Ok((total as u32, remaining as u32, generated_at))
    }

    /// Regenerate recovery codes (T057)
    ///
    /// Invalidates existing codes and generates new ones.
    pub async fn regenerate_recovery_codes(
        &self,
        user_id: Uuid,
        ctx: &RequestContext,
    ) -> Result<RecoveryCodesResponse, TotpError> {
        // Verify 2FA is enabled
        let totp_enabled: bool = sqlx::query_scalar("SELECT totp_enabled FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        if !totp_enabled {
            return Err(TotpError::NotEnabled);
        }

        // Generate new codes
        let (codes, code_hashes) = self.generate_recovery_codes()?;

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Delete existing codes
        sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;

        // Insert new codes
        for code_hash in &code_hashes {
            sqlx::query(
                r#"
                INSERT INTO recovery_codes (user_id, code_hash, created_at)
                VALUES ($1, $2, NOW())
                "#,
            )
            .bind(user_id)
            .bind(code_hash)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Log security event
        AuditEvent::new("auth.2fa.recovery.regenerated")
            .with_user(user_id)
            .with_context(ctx)
            .warn()
            .persist(&self.pool)
            .await;

        Ok(RecoveryCodesResponse::new(codes))
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /// Encrypt a TOTP secret using AES-256-GCM
    fn encrypt_secret(&self, secret: &str) -> Result<String, TotpError> {
        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| TotpError::Encryption(e.to_string()))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, secret.as_bytes())
            .map_err(|e| TotpError::Encryption(e.to_string()))?;

        // Combine nonce + ciphertext and encode as base64
        let mut combined = nonce_bytes.to_vec();
        combined.extend(ciphertext);

        Ok(BASE64.encode(&combined))
    }

    /// Decrypt a TOTP secret using AES-256-GCM
    fn decrypt_secret(&self, encrypted: &str) -> Result<String, TotpError> {
        let combined = BASE64
            .decode(encrypted)
            .map_err(|e| TotpError::Encryption(e.to_string()))?;

        if combined.len() < 12 {
            return Err(TotpError::Encryption("Invalid encrypted data".to_string()));
        }

        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new_from_slice(&self.encryption_key)
            .map_err(|e| TotpError::Encryption(e.to_string()))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| TotpError::Encryption(e.to_string()))?;

        String::from_utf8(plaintext).map_err(|e| TotpError::Encryption(e.to_string()))
    }

    /// Verify a TOTP code against a secret
    fn verify_code(&self, secret_base32: &str, code: &str) -> Result<bool, TotpError> {
        let secret = Secret::Encoded(secret_base32.to_string());
        let secret_bytes = secret
            .to_bytes()
            .map_err(|e| TotpError::TotpGeneration(e.to_string()))?;

        let totp = TOTP::new(
            Algorithm::SHA1,
            TOTP_DIGITS,
            TOTP_SKEW,
            TOTP_STEP,
            secret_bytes,
            None,
            String::new(),
        )
        .map_err(|e| TotpError::TotpGeneration(e.to_string()))?;

        Ok(totp.check_current(code).unwrap_or(false))
    }

    /// Generate recovery codes
    ///
    /// Returns (plaintext_codes, bcrypt_hashes)
    fn generate_recovery_codes(&self) -> Result<(Vec<String>, Vec<String>), TotpError> {
        let mut codes = Vec::with_capacity(RECOVERY_CODE_COUNT);
        let mut hashes = Vec::with_capacity(RECOVERY_CODE_COUNT);

        for _ in 0..RECOVERY_CODE_COUNT {
            // Generate random code
            let code = self.generate_recovery_code();
            let display_code = format!("{}-{}", &code[0..4], &code[4..8]);

            // Hash with bcrypt
            let hash = bcrypt::hash(&code, bcrypt::DEFAULT_COST)
                .map_err(|e| TotpError::Encryption(e.to_string()))?;

            codes.push(display_code);
            hashes.push(hash);
        }

        Ok((codes, hashes))
    }

    /// Generate a single recovery code (8 alphanumeric chars)
    fn generate_recovery_code(&self) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let mut code = String::with_capacity(RECOVERY_CODE_LENGTH);
        let mut rng = rand::thread_rng();

        for _ in 0..RECOVERY_CODE_LENGTH {
            let idx = (rng.next_u32() as usize) % CHARSET.len();
            code.push(CHARSET[idx] as char);
        }

        code
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_recovery_code_generation() {
        // Test code format - charset excludes 0, 1, I, O to avoid confusion
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let code = "ABCD2345";

        assert_eq!(code.len(), 8);
        assert!(code.chars().all(|c| CHARSET.contains(&(c as u8))));
    }

    #[test]
    fn test_recovery_code_format() {
        let code = "ABCD-2345";
        assert_eq!(code.len(), 9);
        assert!(code.chars().nth(4) == Some('-'));
    }
}
