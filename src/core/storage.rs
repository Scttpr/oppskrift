//! S3-compatible storage client
//!
//! Supports both AWS S3 and S3-compatible services (MinIO, DigitalOcean Spaces, etc.)

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{config::Region, primitives::ByteStream, Client};
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// S3 bucket name
    pub bucket: String,
    /// Optional custom endpoint (for S3-compatible services)
    pub endpoint: Option<String>,
    /// AWS region
    pub region: String,
    /// Public URL prefix for accessing files
    pub public_url_prefix: String,
}

impl StorageConfig {
    /// Create config from environment variables
    pub fn from_env() -> AppResult<Self> {
        let bucket = std::env::var("S3_BUCKET")
            .map_err(|_| AppError::Internal("S3_BUCKET not set".to_string()))?;

        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        let endpoint = std::env::var("S3_ENDPOINT").ok();

        let public_url_prefix = std::env::var("S3_PUBLIC_URL")
            .unwrap_or_else(|_| format!("https://{}.s3.{}.amazonaws.com", bucket, region));

        Ok(Self {
            bucket,
            endpoint,
            region,
            public_url_prefix,
        })
    }
}

/// Storage client for file uploads
#[derive(Clone)]
pub struct StorageClient {
    client: Client,
    config: StorageConfig,
}

impl StorageClient {
    /// Create a new storage client from configuration
    pub async fn new(config: StorageConfig) -> AppResult<Self> {
        let region_provider = RegionProviderChain::first_try(Region::new(config.region.clone()));

        let mut aws_config =
            aws_config::defaults(aws_config::BehaviorVersion::latest()).region(region_provider);

        // Use custom endpoint for S3-compatible services
        if let Some(endpoint) = &config.endpoint {
            aws_config = aws_config.endpoint_url(endpoint);
        }

        // Honor explicit S3_ACCESS_KEY_ID / S3_SECRET_ACCESS_KEY (e.g. MinIO);
        // otherwise fall back to the default AWS credential provider chain.
        if let (Ok(access_key), Ok(secret_key)) = (
            std::env::var("S3_ACCESS_KEY_ID"),
            std::env::var("S3_SECRET_ACCESS_KEY"),
        ) {
            aws_config = aws_config.credentials_provider(aws_sdk_s3::config::Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "oppskrift-env",
            ));
        }

        let aws_config = aws_config.load().await;
        let client = Client::new(&aws_config);

        Ok(Self { client, config })
    }

    /// Create a new storage client from environment variables
    pub async fn from_env() -> AppResult<Self> {
        let config = StorageConfig::from_env()?;
        Self::new(config).await
    }

    /// Upload a file to storage
    pub async fn upload(&self, key: &str, data: Vec<u8>, content_type: &str) -> AppResult<String> {
        let body = ByteStream::from(data);

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to upload file: {}", e)))?;

        Ok(format!("{}/{}", self.config.public_url_prefix, key))
    }

    /// Delete a file from storage
    pub async fn delete(&self, key: &str) -> AppResult<()> {
        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to delete file: {}", e)))?;

        Ok(())
    }

    /// Generate a unique key for a recipe image
    pub fn generate_image_key(recipe_id: Uuid, extension: &str) -> String {
        let file_id = Uuid::new_v4();
        format!("recipes/{}/images/{}.{}", recipe_id, file_id, extension)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_image_key() {
        let recipe_id = Uuid::new_v4();
        let key = StorageClient::generate_image_key(recipe_id, "jpg");

        assert!(key.starts_with("recipes/"));
        assert!(key.contains(&recipe_id.to_string()));
        assert!(key.ends_with(".jpg"));
    }
}
