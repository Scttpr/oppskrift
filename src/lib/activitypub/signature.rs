//! HTTP Signature support for ActivityPub
//!
//! Implements HTTP Signatures for authenticating ActivityPub requests.
//! See: https://docs.joinmastodon.org/spec/security/

use base64::Engine;
use chrono::Utc;
use rsa::{
    pkcs8::DecodePublicKey,
    sha2::Sha256,
    signature::Verifier,
    RsaPublicKey,
};
use serde::{Deserialize, Serialize};

use crate::lib::error::{AppError, AppResult};

/// HTTP Signature header components
#[derive(Debug, Clone)]
pub struct HttpSignature {
    pub key_id: String,
    pub algorithm: String,
    pub headers: Vec<String>,
    pub signature: String,
}

impl HttpSignature {
    /// Parse an HTTP Signature header value
    pub fn parse(header: &str) -> Option<Self> {
        let mut key_id = None;
        let mut algorithm = None;
        let mut headers = None;
        let mut signature = None;

        for part in header.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                let value = value.trim_matches('"');
                match key {
                    "keyId" => key_id = Some(value.to_string()),
                    "algorithm" => algorithm = Some(value.to_string()),
                    "headers" => {
                        headers = Some(value.split_whitespace().map(String::from).collect())
                    }
                    "signature" => signature = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        Some(Self {
            key_id: key_id?,
            algorithm: algorithm.unwrap_or_else(|| "rsa-sha256".to_string()),
            headers: headers.unwrap_or_else(|| vec!["(request-target)".to_string(), "date".to_string()]),
            signature: signature?,
        })
    }

    /// Build the string to sign based on the signature headers
    pub fn build_signing_string(
        &self,
        method: &str,
        path: &str,
        headers: &[(String, String)],
    ) -> String {
        let mut parts = Vec::new();

        for header_name in &self.headers {
            if header_name == "(request-target)" {
                parts.push(format!("(request-target): {} {}", method.to_lowercase(), path));
            } else {
                if let Some((_, value)) = headers.iter().find(|(k, _)| k.to_lowercase() == *header_name) {
                    parts.push(format!("{}: {}", header_name, value));
                }
            }
        }

        parts.join("\n")
    }

    /// Format as an HTTP header value
    pub fn to_header_value(&self) -> String {
        format!(
            r#"keyId="{}",algorithm="{}",headers="{}",signature="{}""#,
            self.key_id,
            self.algorithm,
            self.headers.join(" "),
            self.signature
        )
    }
}

/// Signing context for outgoing requests
#[derive(Debug, Clone)]
pub struct SigningContext {
    pub key_id: String,
    pub private_key_pem: String,
}

impl SigningContext {
    /// Create a new signing context
    pub fn new(key_id: String, private_key_pem: String) -> Self {
        Self {
            key_id,
            private_key_pem,
        }
    }

    /// Sign a request and return the Signature header value
    /// Note: Actual signing requires a crypto library - this is a placeholder
    pub fn sign_request(
        &self,
        method: &str,
        path: &str,
        host: &str,
        body_digest: Option<&str>,
    ) -> SignedHeaders {
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        let mut headers_to_sign = vec![
            "(request-target)".to_string(),
            "host".to_string(),
            "date".to_string(),
        ];

        let mut header_values = vec![
            ("host".to_string(), host.to_string()),
            ("date".to_string(), date.clone()),
        ];

        if let Some(digest) = body_digest {
            headers_to_sign.push("digest".to_string());
            header_values.push(("digest".to_string(), format!("SHA-256={}", digest)));
        }

        // Build signing string
        let signing_string = self.build_signing_string(method, path, &headers_to_sign, &header_values);

        // TODO: Actual RSA-SHA256 signing with private key
        // For now, create a placeholder signature
        let signature = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("signed:{}", signing_string).as_bytes(),
        );

        let sig = HttpSignature {
            key_id: self.key_id.clone(),
            algorithm: "rsa-sha256".to_string(),
            headers: headers_to_sign,
            signature,
        };

        SignedHeaders {
            signature: sig.to_header_value(),
            date,
            digest: body_digest.map(|d| format!("SHA-256={}", d)),
        }
    }

    fn build_signing_string(
        &self,
        method: &str,
        path: &str,
        header_names: &[String],
        header_values: &[(String, String)],
    ) -> String {
        let mut parts = Vec::new();

        for name in header_names {
            if name == "(request-target)" {
                parts.push(format!("(request-target): {} {}", method.to_lowercase(), path));
            } else {
                if let Some((_, value)) = header_values.iter().find(|(k, _)| k == name) {
                    parts.push(format!("{}: {}", name, value));
                }
            }
        }

        parts.join("\n")
    }
}

/// Headers needed for a signed request
#[derive(Debug, Clone)]
pub struct SignedHeaders {
    pub signature: String,
    pub date: String,
    pub digest: Option<String>,
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub key_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl VerificationResult {
    pub fn success(key_id: String) -> Self {
        Self {
            valid: true,
            key_id,
            error: None,
        }
    }

    pub fn failure(key_id: String, error: String) -> Self {
        Self {
            valid: false,
            key_id,
            error: Some(error),
        }
    }
}

/// Remote actor with public key
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteActor {
    pub id: String,
    #[serde(rename = "publicKey")]
    pub public_key: Option<PublicKeyInfo>,
}

/// Public key information from ActivityPub actor
#[derive(Debug, Clone, Deserialize)]
pub struct PublicKeyInfo {
    pub id: String,
    pub owner: String,
    #[serde(rename = "publicKeyPem")]
    pub public_key_pem: String,
}

/// Fetch an actor from a remote ActivityPub server
///
/// Fetches the actor document from the key_id URL (stripping the #fragment)
/// and extracts the public key for signature verification.
pub async fn fetch_actor(key_id: &str) -> AppResult<RemoteActor> {
    // Extract the actor URL from the key_id (remove #fragment)
    let actor_url = key_id.split('#').next().unwrap_or(key_id);

    let client = reqwest::Client::new();
    let response = client
        .get(actor_url)
        .header("Accept", "application/activity+json, application/ld+json")
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to fetch actor: {}", e)))?;

    if !response.status().is_success() {
        return Err(AppError::Internal(format!(
            "Failed to fetch actor: HTTP {}",
            response.status()
        )));
    }

    let actor: RemoteActor = response
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to parse actor: {}", e)))?;

    Ok(actor)
}

/// Verify an HTTP signature using the actor's public key
///
/// Returns Ok(VerificationResult) indicating whether the signature is valid.
pub async fn verify_signature(
    signature: &HttpSignature,
    method: &str,
    path: &str,
    headers: &[(String, String)],
) -> AppResult<VerificationResult> {
    // Fetch the actor to get the public key
    let actor = fetch_actor(&signature.key_id).await?;

    let public_key_info = actor.public_key.ok_or_else(|| {
        AppError::Unauthorized("Actor has no public key".to_string())
    })?;

    // Verify key_id matches
    if public_key_info.id != signature.key_id {
        return Ok(VerificationResult::failure(
            signature.key_id.clone(),
            "Key ID mismatch".to_string(),
        ));
    }

    // Parse the public key
    let public_key = RsaPublicKey::from_public_key_pem(&public_key_info.public_key_pem)
        .map_err(|e| AppError::Internal(format!("Failed to parse public key: {}", e)))?;

    // Build the signing string
    let signing_string = signature.build_signing_string(method, path, headers);

    // Decode the signature
    let signature_bytes = base64::engine::general_purpose::STANDARD
        .decode(&signature.signature)
        .map_err(|e| AppError::Internal(format!("Failed to decode signature: {}", e)))?;

    // Verify the signature using PKCS1v15 with SHA-256
    let verifying_key = rsa::pkcs1v15::VerifyingKey::<Sha256>::new(public_key);
    let rsa_signature = rsa::pkcs1v15::Signature::try_from(signature_bytes.as_slice())
        .map_err(|e| AppError::Internal(format!("Invalid signature format: {}", e)))?;

    match verifying_key.verify(signing_string.as_bytes(), &rsa_signature) {
        Ok(()) => Ok(VerificationResult::success(signature.key_id.clone())),
        Err(e) => Ok(VerificationResult::failure(
            signature.key_id.clone(),
            format!("Signature verification failed: {}", e),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_signature_header() {
        let header = r#"keyId="https://example.com/users/1#main-key",algorithm="rsa-sha256",headers="(request-target) host date",signature="abc123""#;

        let sig = HttpSignature::parse(header).unwrap();

        assert_eq!(sig.key_id, "https://example.com/users/1#main-key");
        assert_eq!(sig.algorithm, "rsa-sha256");
        assert_eq!(sig.headers, vec!["(request-target)", "host", "date"]);
        assert_eq!(sig.signature, "abc123");
    }

    #[test]
    fn test_signature_to_header() {
        let sig = HttpSignature {
            key_id: "https://example.com/key".to_string(),
            algorithm: "rsa-sha256".to_string(),
            headers: vec!["host".to_string(), "date".to_string()],
            signature: "abc".to_string(),
        };

        let header = sig.to_header_value();
        assert!(header.contains("keyId="));
        assert!(header.contains("algorithm="));
    }
}
