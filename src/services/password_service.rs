//! Password hashing and validation service
//!
//! Implements Argon2id hashing with OWASP-recommended parameters
//! and password strength validation including HIBP breach checking.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};
use lazy_static::lazy_static;
use regex::Regex;
use sha1::{Digest, Sha1};
use thiserror::Error;

lazy_static! {
    /// Password must contain at least one uppercase letter
    static ref HAS_UPPERCASE: Regex = Regex::new(r"[A-Z]").expect("Invalid regex");
    /// Password must contain at least one lowercase letter
    static ref HAS_LOWERCASE: Regex = Regex::new(r"[a-z]").expect("Invalid regex");
    /// Password must contain at least one digit
    static ref HAS_DIGIT: Regex = Regex::new(r"\d").expect("Invalid regex");
}

/// Minimum password length (OWASP recommendation)
pub const MIN_PASSWORD_LENGTH: usize = 10;

/// HIBP API timeout in seconds
const HIBP_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("Le mot de passe est trop court (au moins {MIN_PASSWORD_LENGTH} caractères)")]
    TooShort,
    #[error("Le mot de passe doit contenir au moins une majuscule")]
    NoUppercase,
    #[error("Le mot de passe doit contenir au moins une minuscule")]
    NoLowercase,
    #[error("Le mot de passe doit contenir au moins un chiffre")]
    NoDigit,
    #[error("Ce mot de passe a été trouvé dans des fuites de données et ne peut pas être utilisé")]
    Breached,
    #[error("Password hashing failed")]
    HashingFailed,
    #[error("Password verification failed")]
    VerificationFailed,
    #[error("HIBP API error: {0}")]
    HibpError(String),
}

/// Password service for hashing, verification, and validation
#[derive(Clone)]
pub struct PasswordService {
    argon2: Argon2<'static>,
    hibp_enabled: bool,
}

impl Default for PasswordService {
    fn default() -> Self {
        Self::new(true)
    }
}

impl PasswordService {
    /// Create a new password service
    ///
    /// # Arguments
    /// * `hibp_enabled` - Whether to check passwords against HIBP database
    pub fn new(hibp_enabled: bool) -> Self {
        // OWASP recommended Argon2id parameters (as of 2024):
        // - m (memory): 19456 KiB (19 MiB)
        // - t (iterations): 2
        // - p (parallelism): 1
        let params = Params::new(19456, 2, 1, None).expect("Invalid Argon2 params");

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        Self {
            argon2,
            hibp_enabled,
        }
    }

    /// Validate password strength without hashing
    ///
    /// Checks:
    /// - Minimum length (10 characters)
    /// - Contains uppercase letter
    /// - Contains lowercase letter
    /// - Contains digit
    pub fn validate_strength(&self, password: &str) -> Result<(), PasswordError> {
        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(PasswordError::TooShort);
        }

        if !HAS_UPPERCASE.is_match(password) {
            return Err(PasswordError::NoUppercase);
        }

        if !HAS_LOWERCASE.is_match(password) {
            return Err(PasswordError::NoLowercase);
        }

        if !HAS_DIGIT.is_match(password) {
            return Err(PasswordError::NoDigit);
        }

        Ok(())
    }

    /// Check if password has been found in data breaches using HIBP k-anonymity API
    ///
    /// Uses k-anonymity: only first 5 characters of SHA-1 hash are sent to API.
    /// Returns Ok(false) if password is safe, Ok(true) if breached.
    pub async fn check_hibp(&self, password: &str) -> Result<bool, PasswordError> {
        if !self.hibp_enabled {
            return Ok(false);
        }

        // SHA-1 hash the password
        let mut hasher = Sha1::new();
        hasher.update(password.as_bytes());
        let hash = hasher.finalize();
        let hash_hex = hex::encode(hash).to_uppercase();

        // Split into prefix (first 5 chars) and suffix (rest)
        let (prefix, suffix) = hash_hex.split_at(5);

        // Query HIBP API with k-anonymity
        let url = format!("https://api.pwnedpasswords.com/range/{}", prefix);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(HIBP_TIMEOUT_SECS))
            .build()
            .map_err(|e| PasswordError::HibpError(e.to_string()))?;

        let response = client
            .get(&url)
            .header("User-Agent", "Oppskrift/1.0")
            .send()
            .await
            .map_err(|e| {
                // Don't fail registration if HIBP is down - just log and continue
                tracing::warn!("HIBP API error: {}", e);
                PasswordError::HibpError(e.to_string())
            })?;

        if !response.status().is_success() {
            tracing::warn!("HIBP API returned status: {}", response.status());
            // Don't block registration on API errors
            return Ok(false);
        }

        let body = response.text().await.map_err(|e| {
            tracing::warn!("HIBP API response error: {}", e);
            PasswordError::HibpError(e.to_string())
        })?;

        // Check if our suffix is in the response
        // Response format: SUFFIX:COUNT\r\n
        for line in body.lines() {
            if let Some((hash_suffix, _count)) = line.split_once(':') {
                if hash_suffix == suffix {
                    return Ok(true); // Password found in breach
                }
            }
        }

        Ok(false) // Password not found in breaches
    }

    /// Hash a password using Argon2id
    ///
    /// Returns the PHC-formatted hash string suitable for storage.
    pub async fn hash(&self, password: &str) -> Result<String, PasswordError> {
        let argon2 = self.argon2.clone();
        let password = password.to_owned();

        // Argon2 hashing is CPU-bound; run off the async runtime.
        tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);

            let hash = argon2
                .hash_password(password.as_bytes(), &salt)
                .map_err(|_| PasswordError::HashingFailed)?;

            Ok(hash.to_string())
        })
        .await
        .map_err(|_| PasswordError::HashingFailed)?
    }

    /// Verify a password against a stored hash
    ///
    /// Performs constant-time comparison to prevent timing attacks.
    pub async fn verify(&self, password: &str, hash: &str) -> Result<bool, PasswordError> {
        let argon2 = self.argon2.clone();
        let password = password.to_owned();
        let hash = hash.to_owned();

        // Argon2 verification is CPU-bound; run off the async runtime.
        tokio::task::spawn_blocking(move || {
            let parsed_hash =
                PasswordHash::new(&hash).map_err(|_| PasswordError::VerificationFailed)?;

            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(()) => Ok(true),
                Err(argon2::password_hash::Error::Password) => Ok(false),
                Err(_) => Err(PasswordError::VerificationFailed),
            }
        })
        .await
        .map_err(|_| PasswordError::VerificationFailed)?
    }

    /// Generate a fake hash for timing attack prevention
    ///
    /// When a user doesn't exist, we still want to perform a hash verification
    /// to prevent timing attacks that could reveal which emails are registered.
    pub async fn fake_verify(&self, password: &str) {
        // Use a pre-generated hash that will always fail verification
        // This ensures consistent timing regardless of user existence
        let fake_hash = "$argon2id$v=19$m=19456,t=2,p=1$fakesalt00000000$fakehash0000000000000000000000000000000000";

        // Ignore result - we just want the timing
        let _ = self.verify(password, fake_hash).await;
    }

    /// Validate password and check HIBP in one call
    ///
    /// Use this for registration and password change flows.
    pub async fn validate_new_password(&self, password: &str) -> Result<(), PasswordError> {
        // First check strength requirements
        self.validate_strength(password)?;

        // Then check HIBP (only if enabled)
        if self.check_hibp(password).await? {
            return Err(PasswordError::Breached);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_strength_valid() {
        let service = PasswordService::new(false);
        assert!(service.validate_strength("SecurePass1").is_ok());
        assert!(service.validate_strength("MyPassword123").is_ok());
        assert!(service.validate_strength("Test1234567").is_ok());
    }

    #[test]
    fn test_password_too_short() {
        let service = PasswordService::new(false);
        assert!(matches!(
            service.validate_strength("Short1A"),
            Err(PasswordError::TooShort)
        ));
    }

    #[test]
    fn test_password_no_uppercase() {
        let service = PasswordService::new(false);
        assert!(matches!(
            service.validate_strength("nouppercase1"),
            Err(PasswordError::NoUppercase)
        ));
    }

    #[test]
    fn test_password_no_lowercase() {
        let service = PasswordService::new(false);
        assert!(matches!(
            service.validate_strength("NOLOWERCASE1"),
            Err(PasswordError::NoLowercase)
        ));
    }

    #[test]
    fn test_password_no_digit() {
        let service = PasswordService::new(false);
        assert!(matches!(
            service.validate_strength("NoDigitsHere"),
            Err(PasswordError::NoDigit)
        ));
    }

    #[tokio::test]
    async fn test_hash_and_verify() {
        let service = PasswordService::new(false);
        let password = "SecurePassword123";

        let hash = service
            .hash(password)
            .await
            .expect("Hashing should succeed");
        assert!(hash.starts_with("$argon2id$"));

        assert!(service
            .verify(password, &hash)
            .await
            .expect("Verification should succeed"));
        assert!(!service
            .verify("WrongPassword123", &hash)
            .await
            .expect("Verification should succeed"));
    }
}
