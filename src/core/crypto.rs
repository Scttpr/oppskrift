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
