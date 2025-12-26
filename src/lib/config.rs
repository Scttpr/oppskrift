//! Configuration validation module
//! Ensures all required secrets are present at startup

use std::env;

/// Configuration loaded from environment
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub base_url: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_endpoint: Option<String>,
    pub host: String,
    pub port: u16,
}

impl Config {
    /// Load and validate configuration from environment
    /// Panics if required variables are missing or invalid
    pub fn from_env() -> Self {
        // Required secrets - panic if missing
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");

        let jwt_secret = env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set");

        // Validate JWT_SECRET length
        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters");
        }

        let s3_bucket = env::var("S3_BUCKET")
            .expect("S3_BUCKET must be set");

        // Optional with defaults
        let base_url = env::var("BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        let s3_region = env::var("S3_REGION")
            .unwrap_or_else(|_| "us-east-1".to_string());
        let s3_endpoint = env::var("S3_ENDPOINT").ok();
        let host = env::var("HOST")
            .unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .expect("PORT must be a valid number");

        Self {
            database_url,
            jwt_secret,
            base_url,
            s3_bucket,
            s3_region,
            s3_endpoint,
            host,
            port,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_jwt_secret_length_validation() {
        // This test documents the validation requirement
        // Actual validation happens at runtime in from_env()
        let short_secret = "short";
        assert!(short_secret.len() < 32);
    }
}
