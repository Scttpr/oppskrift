//! CSRF (Cross-Site Request Forgery) protection utilities
//!
//! Provides token generation and validation for form submissions.
//! Uses HMAC-SHA256 with a secret key to generate and verify tokens.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};

/// CSRF token TTL (1 hour)
const TOKEN_TTL_HOURS: i64 = 1;

/// CSRF token length in bytes
const TOKEN_RANDOM_BYTES: usize = 32;

type HmacSha256 = Hmac<Sha256>;

/// CSRF token with metadata
#[derive(Debug, Clone)]
pub struct CsrfToken {
    /// The token string (Base64-URL encoded)
    pub token: String,
    /// When the token expires
    pub expires_at: DateTime<Utc>,
}

/// Generate a CSRF token for a session
///
/// The token contains:
/// - Random bytes for uniqueness
/// - Session ID binding
/// - Timestamp for expiration
/// - HMAC signature for integrity
pub fn generate_csrf_token(session_id: Uuid, secret: &[u8]) -> AppResult<CsrfToken> {
    let expires_at = Utc::now() + Duration::hours(TOKEN_TTL_HOURS);
    let timestamp = expires_at.timestamp();

    // Generate random bytes
    let mut random_bytes = [0u8; TOKEN_RANDOM_BYTES];
    rand::thread_rng().fill_bytes(&mut random_bytes);

    // Create token payload: random || session_id || timestamp
    let mut payload = Vec::with_capacity(TOKEN_RANDOM_BYTES + 16 + 8);
    payload.extend_from_slice(&random_bytes);
    payload.extend_from_slice(session_id.as_bytes());
    payload.extend_from_slice(&timestamp.to_be_bytes());

    // Sign the payload
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|e| AppError::Internal(format!("Invalid HMAC key: {}", e)))?;
    mac.update(&payload);
    let signature = mac.finalize().into_bytes();

    // Combine payload and signature
    let mut token_bytes = payload;
    token_bytes.extend_from_slice(&signature);

    // Encode as URL-safe Base64
    let token = URL_SAFE_NO_PAD.encode(&token_bytes);

    Ok(CsrfToken { token, expires_at })
}

/// Validate a CSRF token
///
/// Checks:
/// 1. Token format and signature integrity
/// 2. Session ID matches
/// 3. Token hasn't expired
pub fn validate_csrf_token(token: &str, session_id: Uuid, secret: &[u8]) -> AppResult<()> {
    // Decode Base64
    let token_bytes = URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|_| AppError::Unauthorized("Invalid CSRF token format".to_string()))?;

    // Expected size: random (32) + session_id (16) + timestamp (8) + signature (32) = 88
    const EXPECTED_SIZE: usize = TOKEN_RANDOM_BYTES + 16 + 8 + 32;
    if token_bytes.len() != EXPECTED_SIZE {
        return Err(AppError::Unauthorized(
            "Invalid CSRF token length".to_string(),
        ));
    }

    // Split into payload and signature
    let (payload, signature) = token_bytes.split_at(TOKEN_RANDOM_BYTES + 16 + 8);

    // Verify signature
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|e| AppError::Internal(format!("Invalid HMAC key: {}", e)))?;
    mac.update(payload);
    mac.verify_slice(signature)
        .map_err(|_| AppError::Unauthorized("Invalid CSRF token signature".to_string()))?;

    // Extract session ID and timestamp
    let token_session_bytes: [u8; 16] = payload[TOKEN_RANDOM_BYTES..TOKEN_RANDOM_BYTES + 16]
        .try_into()
        .map_err(|_| AppError::Unauthorized("Invalid CSRF token".to_string()))?;
    let token_session_id = Uuid::from_bytes(token_session_bytes);

    let timestamp_bytes: [u8; 8] = payload[TOKEN_RANDOM_BYTES + 16..]
        .try_into()
        .map_err(|_| AppError::Unauthorized("Invalid CSRF token".to_string()))?;
    let timestamp = i64::from_be_bytes(timestamp_bytes);

    // Check session ID matches
    if token_session_id != session_id {
        return Err(AppError::Unauthorized(
            "CSRF token session mismatch".to_string(),
        ));
    }

    // Check expiration
    let expires_at = DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| AppError::Unauthorized("Invalid CSRF token timestamp".to_string()))?;

    if Utc::now() > expires_at {
        return Err(AppError::Unauthorized("CSRF token expired".to_string()));
    }

    Ok(())
}

/// Hidden input HTML for CSRF token in forms
pub fn csrf_input(token: &str) -> String {
    format!(
        r#"<input type="hidden" name="_csrf" value="{}" />"#,
        html_escape::encode_double_quoted_attribute(token)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_secret() -> Vec<u8> {
        b"test-secret-key-at-least-32-bytes-long".to_vec()
    }

    #[test]
    fn test_generate_csrf_token() {
        let session_id = Uuid::new_v4();
        let secret = test_secret();

        let result = generate_csrf_token(session_id, &secret);
        assert!(result.is_ok());

        let csrf = result.unwrap();
        assert!(!csrf.token.is_empty());
        assert!(csrf.expires_at > Utc::now());
    }

    #[test]
    fn test_validate_csrf_token_success() {
        let session_id = Uuid::new_v4();
        let secret = test_secret();

        let csrf = generate_csrf_token(session_id, &secret).unwrap();
        let result = validate_csrf_token(&csrf.token, session_id, &secret);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_csrf_token_wrong_session() {
        let session_id = Uuid::new_v4();
        let other_session = Uuid::new_v4();
        let secret = test_secret();

        let csrf = generate_csrf_token(session_id, &secret).unwrap();
        let result = validate_csrf_token(&csrf.token, other_session, &secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_csrf_token_wrong_secret() {
        let session_id = Uuid::new_v4();
        let secret = test_secret();
        let wrong_secret = b"wrong-secret-key-at-least-32-bytes-long";

        let csrf = generate_csrf_token(session_id, &secret).unwrap();
        let result = validate_csrf_token(&csrf.token, session_id, wrong_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_csrf_token_invalid_format() {
        let session_id = Uuid::new_v4();
        let secret = test_secret();

        let result = validate_csrf_token("not-a-valid-token", session_id, &secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_csrf_input_escapes_html() {
        let token = "test<>&\"token";
        let input = csrf_input(token);
        assert!(input.contains("&lt;"));
        assert!(input.contains("&gt;"));
        assert!(input.contains("&amp;"));
        assert!(input.contains("&quot;"));
    }

    #[test]
    fn test_tokens_are_unique() {
        let session_id = Uuid::new_v4();
        let secret = test_secret();

        let csrf1 = generate_csrf_token(session_id, &secret).unwrap();
        let csrf2 = generate_csrf_token(session_id, &secret).unwrap();

        assert_ne!(csrf1.token, csrf2.token, "Tokens should be unique");
    }
}
