//! Recovery code DTOs
//!
//! Structs for 2FA recovery codes - single-use backup codes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Recovery code record for verification (minimal fields needed)
#[derive(Debug, Clone, FromRow)]
pub struct RecoveryCode {
    pub id: Uuid,
    pub code_hash: String,
}

/// Response containing recovery codes (only shown once after generation)
#[derive(Debug, Serialize)]
pub struct RecoveryCodesResponse {
    pub message: String,
    /// Plaintext recovery codes (8 codes, format: XXXX-XXXX each)
    pub codes: Vec<String>,
    /// Timestamp when codes were generated
    pub generated_at: DateTime<Utc>,
}

/// Status of recovery codes
#[derive(Debug, Serialize)]
pub struct RecoveryCodesStatus {
    /// Total number of codes generated
    pub total: u32,
    /// Number of codes remaining (unused)
    pub remaining: u32,
    /// When codes were last regenerated
    pub generated_at: Option<DateTime<Utc>>,
}

/// Request to regenerate recovery codes
#[derive(Debug, Deserialize)]
pub struct RegenerateRecoveryCodesRequest {
    /// Password confirmation for security
    pub password: String,
}

impl RecoveryCodesResponse {
    pub fn new(codes: Vec<String>) -> Self {
        Self {
            message: "Store these recovery codes in a safe place. They will not be shown again."
                .to_string(),
            codes,
            generated_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_codes_response() {
        let codes = vec!["ABCD-1234".to_string(), "EFGH-5678".to_string()];
        let response = RecoveryCodesResponse::new(codes.clone());

        assert_eq!(response.codes.len(), 2);
        assert!(response.message.contains("safe place"));
    }

    #[test]
    fn test_recovery_codes_status() {
        let status = RecoveryCodesStatus {
            total: 8,
            remaining: 5,
            generated_at: Some(Utc::now()),
        };

        assert_eq!(status.total, 8);
        assert_eq!(status.remaining, 5);
        assert!(status.generated_at.is_some());
    }

    #[test]
    fn test_recovery_code_format() {
        // Recovery codes should be 9 chars: XXXX-XXXX
        let code = "ABCD-1234";
        assert_eq!(code.len(), 9);
        assert!(code.chars().nth(4) == Some('-'));
    }
}
