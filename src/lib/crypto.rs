//! Cryptographic utilities for ActivityPub federation
//! RSA key generation and management

use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding},
};

use crate::lib::error::{AppError, AppResult};

/// RSA key size in bits (2048 is standard for ActivityPub)
const RSA_KEY_SIZE: usize = 2048;

/// Generated RSA keypair in PEM format
#[derive(Debug, Clone)]
pub struct RsaKeypair {
    pub public_key_pem: String,
    pub private_key_pem: String,
}

/// Generate a new RSA keypair for ActivityPub HTTP Signatures
pub fn generate_rsa_keypair() -> AppResult<RsaKeypair> {
    let mut rng = rand::thread_rng();

    // Generate private key
    let private_key = RsaPrivateKey::new(&mut rng, RSA_KEY_SIZE)
        .map_err(|e| AppError::Internal(format!("Failed to generate RSA key: {}", e)))?;

    // Derive public key
    let public_key = RsaPublicKey::from(&private_key);

    // Encode to PEM format
    let private_key_pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("Failed to encode private key: {}", e)))?
        .to_string();

    let public_key_pem = public_key
        .to_public_key_pem(LineEnding::LF)
        .map_err(|e| AppError::Internal(format!("Failed to encode public key: {}", e)))?;

    Ok(RsaKeypair {
        public_key_pem,
        private_key_pem,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_rsa_keypair() {
        let keypair = generate_rsa_keypair().expect("Failed to generate keypair");

        // Check PEM format
        assert!(
            keypair
                .public_key_pem
                .starts_with("-----BEGIN PUBLIC KEY-----")
        );
        assert!(
            keypair
                .private_key_pem
                .starts_with("-----BEGIN PRIVATE KEY-----")
        );

        // Check they end properly
        assert!(
            keypair
                .public_key_pem
                .trim()
                .ends_with("-----END PUBLIC KEY-----")
        );
        assert!(
            keypair
                .private_key_pem
                .trim()
                .ends_with("-----END PRIVATE KEY-----")
        );
    }
}
