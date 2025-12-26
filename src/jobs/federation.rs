//! Federation delivery jobs
//!
//! Handles async delivery of ActivityPub activities to remote inboxes.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::lib::activitypub::SigningContext;

/// A federation delivery job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationJob {
    /// Unique job ID
    pub id: Uuid,
    /// The activity JSON to deliver
    pub activity: serde_json::Value,
    /// Target inbox URLs
    pub inbox_urls: Vec<String>,
    /// Actor ID for signing
    pub actor_id: Uuid,
    /// Retry count
    pub retry_count: u32,
}

impl FederationJob {
    /// Create a new federation job
    pub fn new(activity: serde_json::Value, inbox_urls: Vec<String>, actor_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            activity,
            inbox_urls,
            actor_id,
            retry_count: 0,
        }
    }
}

/// Worker that processes federation delivery jobs
pub struct FederationWorker {
    pool: Arc<PgPool>,
    client: Client,
}

impl FederationWorker {
    /// Create a new federation worker
    pub fn new(pool: Arc<PgPool>) -> Self {
        let client = Client::builder()
            .user_agent("Oppskrift/0.1.0 (ActivityPub)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { pool, client }
    }

    /// Run the worker, processing jobs from the channel
    pub async fn run(self, mut rx: mpsc::Receiver<FederationJob>) {
        tracing::info!("Federation worker started");

        while let Some(job) = rx.recv().await {
            self.process_job(job).await;
        }

        tracing::info!("Federation worker stopped");
    }

    /// Process a single federation job
    async fn process_job(&self, job: FederationJob) {
        tracing::info!(
            job_id = %job.id,
            inbox_count = job.inbox_urls.len(),
            "Processing federation job"
        );

        for inbox_url in &job.inbox_urls {
            match self.deliver_to_inbox(inbox_url, &job).await {
                Ok(()) => {
                    tracing::info!(
                        job_id = %job.id,
                        inbox = %inbox_url,
                        "Successfully delivered activity"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        job_id = %job.id,
                        inbox = %inbox_url,
                        error = %e,
                        "Failed to deliver activity"
                    );
                    // TODO: Queue for retry with exponential backoff
                }
            }
        }
    }

    /// Deliver an activity to a specific inbox
    async fn deliver_to_inbox(
        &self,
        inbox_url: &str,
        job: &FederationJob,
    ) -> Result<(), DeliveryError> {
        // Parse inbox URL
        let url =
            reqwest::Url::parse(inbox_url).map_err(|e| DeliveryError::InvalidUrl(e.to_string()))?;

        let host = url
            .host_str()
            .ok_or_else(|| DeliveryError::InvalidUrl("No host in URL".to_string()))?;
        let path = url.path();

        // Serialize activity
        let body = serde_json::to_string(&job.activity)
            .map_err(|e| DeliveryError::Serialization(e.to_string()))?;

        // Calculate body digest
        let digest = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            ring_compat_sha256(&body),
        );

        // Get signing context for the actor
        // TODO: Retrieve actual private key from database
        let base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let key_id = format!("{}/users/{}#main-key", base_url, job.actor_id);
        let signing_context = SigningContext::new(
            key_id,
            "placeholder_private_key".to_string(), // TODO: Get from DB
        );

        // Sign the request
        let signed = signing_context.sign_request("POST", path, host, Some(&digest));

        // Send the request
        let response = self
            .client
            .post(inbox_url)
            .header("Content-Type", "application/activity+json")
            .header("Accept", "application/activity+json")
            .header("Signature", &signed.signature)
            .header("Date", &signed.date)
            .header("Digest", format!("SHA-256={}", digest))
            .header("Host", host)
            .body(body)
            .send()
            .await
            .map_err(|e| DeliveryError::Network(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(DeliveryError::HttpStatus(response.status().as_u16()))
        }
    }
}

/// Compute SHA-256 hash of data (placeholder - would use ring or sha2 crate)
fn ring_compat_sha256(data: &str) -> Vec<u8> {
    // TODO: Use actual SHA-256 from ring or sha2 crate
    // For now, return a placeholder
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let hash = hasher.finish();
    hash.to_be_bytes().to_vec()
}

/// Errors that can occur during delivery
#[derive(Debug, thiserror::Error)]
pub enum DeliveryError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("HTTP error status: {0}")]
    HttpStatus(u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_federation_job_creation() {
        let activity = serde_json::json!({
            "type": "Create",
            "actor": "https://example.com/users/1"
        });
        let inboxes = vec!["https://remote.example/inbox".to_string()];
        let actor_id = Uuid::new_v4();

        let job = FederationJob::new(activity, inboxes, actor_id);

        assert_eq!(job.retry_count, 0);
        assert_eq!(job.inbox_urls.len(), 1);
    }
}
