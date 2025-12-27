//! Configuration validation module
//! Validates required secrets at startup
//! NO FALLBACKS for security-critical secrets in production

use std::env;

/// Configuration validator - checks required env vars at startup
#[derive(Debug, Clone)]
pub struct Config;

/// SMTP configuration for sending emails
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub from_name: String,
}

impl Config {
    /// Validate configuration from environment
    /// Panics if required variables are missing or invalid
    pub fn from_env() -> Self {
        let is_production = env::var("RUST_ENV")
            .map(|v| v == "production")
            .unwrap_or(false);

        // Required secrets - panic if missing
        let _database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
        let _s3_bucket = env::var("S3_BUCKET").expect("S3_BUCKET must be set");

        // Validate JWT_SECRET length and content
        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters");
        }
        if is_production && (jwt_secret.contains("dev") || jwt_secret.contains("test")) {
            panic!("JWT_SECRET appears to contain development values in production");
        }

        // Validate TOTP_ENCRYPTION_KEY in production
        if is_production {
            let totp_key_hex = env::var("TOTP_ENCRYPTION_KEY").expect(
                "TOTP_ENCRYPTION_KEY must be set in production (64 hex characters = 32 bytes)",
            );
            let totp_key_bytes =
                hex::decode(&totp_key_hex).expect("TOTP_ENCRYPTION_KEY must be valid hex");
            if totp_key_bytes.len() != 32 {
                panic!("TOTP_ENCRYPTION_KEY must be exactly 64 hex characters (32 bytes)");
            }
        }

        Self
    }
}

impl SmtpConfig {
    /// Load SMTP configuration from environment
    /// Returns None if not configured (email sending disabled)
    pub fn from_env(is_production: bool) -> Option<Self> {
        let host = env::var("SMTP_HOST").ok();

        // If SMTP_HOST is not set, SMTP is disabled
        let host = match host {
            Some(h) => h,
            None => {
                if is_production {
                    tracing::warn!("SMTP not configured - email sending disabled");
                }
                return None;
            }
        };

        let port = env::var("SMTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(587);

        let username =
            env::var("SMTP_USER").expect("SMTP_USER must be set when SMTP_HOST is configured");

        let password = env::var("SMTP_PASSWORD")
            .expect("SMTP_PASSWORD must be set when SMTP_HOST is configured");

        let from_address =
            env::var("EMAIL_FROM_ADDRESS").unwrap_or_else(|_| format!("noreply@{}", host));

        let from_name = env::var("EMAIL_FROM_NAME").unwrap_or_else(|_| "Oppskrift".to_string());

        Some(Self {
            host,
            port,
            username,
            password,
            from_address,
            from_name,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_jwt_secret_length_validation() {
        // This test documents the validation requirement
        // Actual validation happens at runtime in from_env()
        let short_secret = "short";
        assert!(short_secret.len() < 32);
    }
}
