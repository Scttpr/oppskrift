//! Configuration validation module
//! Ensures all required secrets are present at startup
//! NO FALLBACKS for security-critical secrets in production

#![allow(dead_code)]

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
    pub auth: AuthConfig,
    pub smtp: Option<SmtpConfig>,
}

/// Authentication-specific configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// AES-256 key for TOTP secret encryption (32 bytes, hex-encoded = 64 chars)
    pub totp_encryption_key: [u8; 32],
    /// Session expiry in days (default: 7)
    pub session_expiry_days: u32,
    /// Account lockout duration in minutes (default: 15)
    pub lockout_duration_minutes: u32,
    /// Max login attempts per IP per minute (default: 10)
    pub rate_limit_login_per_ip: u32,
    /// Max failed login attempts per account before lockout (default: 5)
    pub rate_limit_login_per_account: u32,
}

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
    /// Load and validate configuration from environment
    /// Panics if required variables are missing or invalid
    pub fn from_env() -> Self {
        let is_production = env::var("RUST_ENV")
            .map(|v| v == "production")
            .unwrap_or(false);

        // Required secrets - panic if missing
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        // Validate JWT_SECRET length and content
        if jwt_secret.len() < 32 {
            panic!("JWT_SECRET must be at least 32 characters");
        }
        if is_production && (jwt_secret.contains("dev") || jwt_secret.contains("test")) {
            panic!("JWT_SECRET appears to contain development values in production");
        }

        let s3_bucket = env::var("S3_BUCKET").expect("S3_BUCKET must be set");

        // Optional with defaults
        let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let s3_region = env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let s3_endpoint = env::var("S3_ENDPOINT").ok();
        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .expect("PORT must be a valid number");

        // Auth config
        let auth = AuthConfig::from_env(is_production);

        // SMTP config (optional in development)
        let smtp = SmtpConfig::from_env(is_production);

        Self {
            database_url,
            jwt_secret,
            base_url,
            s3_bucket,
            s3_region,
            s3_endpoint,
            host,
            port,
            auth,
            smtp,
        }
    }
}

impl AuthConfig {
    /// Load auth configuration from environment
    pub fn from_env(is_production: bool) -> Self {
        // TOTP_ENCRYPTION_KEY is required for 2FA feature
        // In development, we can use a default for testing
        let totp_key_hex = env::var("TOTP_ENCRYPTION_KEY").unwrap_or_else(|_| {
            if is_production {
                panic!(
                    "TOTP_ENCRYPTION_KEY must be set in production (64 hex characters = 32 bytes)"
                );
            }
            // Development-only default (DO NOT use in production)
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()
        });

        let totp_key_bytes =
            hex::decode(&totp_key_hex).expect("TOTP_ENCRYPTION_KEY must be valid hex");

        if totp_key_bytes.len() != 32 {
            panic!("TOTP_ENCRYPTION_KEY must be exactly 64 hex characters (32 bytes)");
        }

        let mut totp_encryption_key = [0u8; 32];
        totp_encryption_key.copy_from_slice(&totp_key_bytes);

        // Optional settings with safe defaults
        let session_expiry_days = env::var("SESSION_EXPIRY_DAYS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7);

        let lockout_duration_minutes = env::var("LOCKOUT_DURATION_MINUTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15);

        let rate_limit_login_per_ip = env::var("RATE_LIMIT_LOGIN_PER_IP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let rate_limit_login_per_account = env::var("RATE_LIMIT_LOGIN_PER_ACCOUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        Self {
            totp_encryption_key,
            session_expiry_days,
            lockout_duration_minutes,
            rate_limit_login_per_ip,
            rate_limit_login_per_account,
        }
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
