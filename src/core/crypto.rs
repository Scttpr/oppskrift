//! Cryptographic utilities for ActivityPub federation
//! RSA key generation and management

use rsa::{
    pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding},
    RsaPrivateKey, RsaPublicKey,
};

use crate::core::error::{AppError, AppResult};

/// RSA key pair for ActivityPub HTTP signatures
pub struct RsaKeyPair {
    pub private_key_pem: String,
    pub public_key_pem: String,
}

/// Generate a new RSA key pair for ActivityPub signatures
pub fn generate_rsa_keypair() -> AppResult<RsaKeyPair> {
    let mut rng = rand::thread_rng();
    let bits = 2048;

    let private_key = RsaPrivateKey::new(&mut rng, bits)
        .map_err(|e| AppError::Internal(format!("Failed to generate RSA key: {}", e)))?;

    let public_key = RsaPublicKey::from(&private_key);

    let private_key_pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("Failed to encode private key: {}", e)))?
        .to_string();

    let public_key_pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("Failed to encode public key: {}", e)))?;

    Ok(RsaKeyPair {
        private_key_pem,
        public_key_pem,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_rsa_keypair() {
        let keypair = generate_rsa_keypair().expect("Should generate keypair");

        // Private key should be PEM encoded
        assert!(
            keypair.private_key_pem.contains("BEGIN PRIVATE KEY"),
            "Private key should be PEM encoded"
        );
        assert!(
            keypair.private_key_pem.contains("END PRIVATE KEY"),
            "Private key should be complete"
        );

        // Public key should be PEM encoded
        assert!(
            keypair.public_key_pem.contains("BEGIN PUBLIC KEY"),
            "Public key should be PEM encoded"
        );
        assert!(
            keypair.public_key_pem.contains("END PUBLIC KEY"),
            "Public key should be complete"
        );
    }

    #[test]
    fn test_keypair_uniqueness() {
        let keypair1 = generate_rsa_keypair().expect("Should generate keypair 1");
        let keypair2 = generate_rsa_keypair().expect("Should generate keypair 2");

        // Each keypair should be unique
        assert_ne!(
            keypair1.private_key_pem, keypair2.private_key_pem,
            "Private keys should be unique"
        );
        assert_ne!(
            keypair1.public_key_pem, keypair2.public_key_pem,
            "Public keys should be unique"
        );
    }

    #[test]
    fn test_keypair_minimum_length() {
        let keypair = generate_rsa_keypair().expect("Should generate keypair");

        // 2048-bit RSA keys have substantial PEM output
        // Private key should be at least 1600 chars in PEM format
        assert!(
            keypair.private_key_pem.len() > 1600,
            "Private key should be substantial (got {} chars)",
            keypair.private_key_pem.len()
        );

        // Public key should be at least 300 chars
        assert!(
            keypair.public_key_pem.len() > 300,
            "Public key should be substantial (got {} chars)",
            keypair.public_key_pem.len()
        );
    }
}
