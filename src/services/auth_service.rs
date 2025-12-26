//! Authentication service
//!
//! Handles user registration, email confirmation, login, and logout.
//! Uses secure session-based authentication with SHA-256 token hashing.

use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::net::IpAddr;
use thiserror::Error;
use uuid::Uuid;

use crate::lib::error::AppError;
use crate::models::{RegisterRequest, RegisterResponse, User, RESERVED_USERNAMES};
use crate::services::{
    EmailService, PasswordService, SecurityLogService, SessionService, UserService,
};

/// Email confirmation token expiry in hours
const EMAIL_CONFIRMATION_EXPIRY_HOURS: i64 = 24;

/// Resend confirmation cooldown in minutes
const RESEND_CONFIRMATION_COOLDOWN_MINUTES: i64 = 5;

/// Token length in bytes (256 bits)
const TOKEN_BYTES: usize = 32;

/// Maximum failed login attempts before lockout
const MAX_FAILED_LOGIN_ATTEMPTS: i32 = 5;

/// Lockout duration in minutes
const LOCKOUT_DURATION_MINUTES: i64 = 15;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Email already registered")]
    EmailExists,
    #[error("Username already taken")]
    UsernameExists,
    #[error("Username is reserved")]
    UsernameReserved,
    #[error("Invalid password: {0}")]
    InvalidPassword(String),
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error("Email already verified")]
    AlreadyVerified,
    #[error("Please wait before requesting another confirmation email")]
    TooManyRequests,
    #[error("User not found")]
    UserNotFound,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Account is locked until {0}")]
    AccountLocked(String),
    #[error("Email not verified")]
    EmailNotVerified,
    #[error("Two-factor authentication required")]
    TwoFactorRequired,
    #[error("Invalid two-factor code")]
    InvalidTwoFactorCode,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Password error: {0}")]
    Password(String),
    #[error("Email error: {0}")]
    Email(String),
    #[error("Session error: {0}")]
    Session(String),
    #[error("Application error: {0}")]
    App(String),
}

impl From<AppError> for AuthError {
    fn from(err: AppError) -> Self {
        match err {
            AppError::NotFound(_) => AuthError::UserNotFound,
            AppError::Conflict(msg) => {
                if msg.contains("email") {
                    AuthError::EmailExists
                } else if msg.contains("sername") {
                    AuthError::UsernameExists
                } else {
                    AuthError::App(msg)
                }
            }
            AppError::Database(e) => AuthError::Database(e),
            AppError::Validation(msg) => AuthError::InvalidPassword(msg),
            _ => AuthError::App(err.to_string()),
        }
    }
}

/// Email confirmation token record from database
#[derive(Debug, sqlx::FromRow)]
struct EmailConfirmationTokenRecord {
    id: Uuid,
    user_id: Option<Uuid>,
    email: String,
    expires_at: DateTime<Utc>,
}

/// Authentication service
pub struct AuthService {
    pool: PgPool,
    password_service: PasswordService,
    email_service: EmailService,
    security_log: SecurityLogService,
    session_expiry_days: u32,
    base_url: String,
}

impl AuthService {
    /// Create a new auth service
    pub fn new(
        pool: PgPool,
        password_service: PasswordService,
        email_service: EmailService,
        base_url: String,
        session_expiry_days: u32,
    ) -> Self {
        let security_log = SecurityLogService::new(pool.clone());

        Self {
            pool,
            password_service,
            email_service,
            security_log,
            session_expiry_days,
            base_url,
        }
    }

    /// Generate a secure random token
    ///
    /// Returns (raw_token_hex, token_hash) where:
    /// - raw_token_hex: The token to send to the client (hex-encoded)
    /// - token_hash: The SHA-256 hash to store in the database
    fn generate_token() -> (String, String) {
        let mut token_bytes = [0u8; TOKEN_BYTES];
        rand::thread_rng().fill_bytes(&mut token_bytes);

        let raw_token = hex::encode(token_bytes);

        // Hash for storage
        let mut hasher = Sha256::new();
        hasher.update(token_bytes);
        let hash = hex::encode(hasher.finalize());

        (raw_token, hash)
    }

    /// Hash a token for lookup
    fn hash_token(token: &str) -> Result<String, AuthError> {
        let token_bytes = hex::decode(token).map_err(|_| AuthError::InvalidToken)?;

        let mut hasher = Sha256::new();
        hasher.update(&token_bytes);
        Ok(hex::encode(hasher.finalize()))
    }

    /// Register a new user (T021)
    ///
    /// Steps:
    /// 1. Validate input (email, username, password)
    /// 2. Check username not reserved
    /// 3. Check email/username uniqueness
    /// 4. Hash password
    /// 5. Create unverified user
    /// 6. Generate confirmation token
    /// 7. Send confirmation email
    pub async fn register(
        &self,
        request: RegisterRequest,
        ip: Option<IpAddr>,
    ) -> Result<RegisterResponse, AuthError> {
        // Check if username is reserved
        let username_lower = request.username.to_lowercase();
        if RESERVED_USERNAMES.contains(&username_lower.as_str()) {
            let _ = self
                .security_log
                .register_failure(&request.email, "reserved_username", ip)
                .await;
            return Err(AuthError::UsernameReserved);
        }

        // Check email availability
        if !UserService::email_available(&self.pool, &request.email).await? {
            let _ = self
                .security_log
                .register_failure(&request.email, "email_exists", ip)
                .await;
            return Err(AuthError::EmailExists);
        }

        // Check username availability
        if !UserService::username_available(&self.pool, &username_lower).await? {
            let _ = self
                .security_log
                .register_failure(&request.email, "username_exists", ip)
                .await;
            return Err(AuthError::UsernameExists);
        }

        // Validate password strength and check HIBP
        if let Err(e) = self
            .password_service
            .validate_new_password(&request.password)
            .await
        {
            let _ = self
                .security_log
                .register_failure(&request.email, "weak_password", ip)
                .await;
            return Err(AuthError::InvalidPassword(e.to_string()));
        }

        // Hash the password
        let password_hash = self
            .password_service
            .hash(&request.password)
            .map_err(|e| AuthError::Password(e.to_string()))?;

        // Generate ActivityPub ID
        let ap_id = format!("{}/users/{}", self.base_url, username_lower);

        // Create user with unverified email
        let create_user = crate::models::CreateUser {
            username: username_lower.clone(),
            email: request.email.clone(),
            password_hash,
            display_name: request
                .display_name
                .unwrap_or_else(|| username_lower.clone()),
            bio: None,
            avatar_url: None,
            measurement_pref: None,
            ap_id,
        };

        let user = UserService::create(&self.pool, create_user).await?;

        // Generate confirmation token
        let (raw_token, token_hash) = Self::generate_token();
        let expires_at = Utc::now() + Duration::hours(EMAIL_CONFIRMATION_EXPIRY_HOURS);

        // Store confirmation token (raw query to avoid SQLx offline cache issues)
        sqlx::query(
            r#"
            INSERT INTO email_confirmation_tokens (user_id, email, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(user.id)
        .bind(&request.email)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        // Send confirmation email (don't fail registration if email fails)
        if let Err(e) = self
            .email_service
            .send_confirmation(&request.email, &raw_token)
            .await
        {
            tracing::warn!(
                user_id = %user.id,
                error = %e,
                "Failed to send confirmation email"
            );
        }

        // Log successful registration
        let _ = self
            .security_log
            .register_success(user.id, &request.email, ip)
            .await;

        Ok(RegisterResponse {
            message: "Registration successful. Please check your email to confirm your account."
                .to_string(),
            user_id: user.id,
        })
    }

    /// Confirm email address (T022)
    ///
    /// Steps:
    /// 1. Validate token format
    /// 2. Find token in database
    /// 3. Check expiration
    /// 4. Mark user email as verified
    /// 5. Delete used token
    /// 6. Log security event
    pub async fn confirm_email(&self, token: &str, ip: Option<IpAddr>) -> Result<Uuid, AuthError> {
        let token_hash = Self::hash_token(token)?;

        // Find the token (raw query)
        let token_record: Option<EmailConfirmationTokenRecord> = sqlx::query_as(
            r#"
            SELECT id, user_id, email, expires_at
            FROM email_confirmation_tokens
            WHERE token_hash = $1
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let token_record = token_record.ok_or(AuthError::InvalidToken)?;

        // Check expiration
        if token_record.expires_at < Utc::now() {
            // Clean up expired token
            sqlx::query("DELETE FROM email_confirmation_tokens WHERE id = $1")
                .bind(token_record.id)
                .execute(&self.pool)
                .await?;
            return Err(AuthError::InvalidToken);
        }

        let user_id = token_record.user_id.ok_or(AuthError::InvalidToken)?;

        // Check if already verified
        let user: User = UserService::get_by_id(&self.pool, user_id).await?;
        if user.email_verified && user.email == token_record.email {
            // Delete the token since it's no longer needed
            sqlx::query("DELETE FROM email_confirmation_tokens WHERE id = $1")
                .bind(token_record.id)
                .execute(&self.pool)
                .await?;
            return Err(AuthError::AlreadyVerified);
        }

        // Update user email and mark as verified
        // This handles both initial verification and email change verification
        sqlx::query(
            r#"
            UPDATE users
            SET email = $2, email_verified = true, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(&token_record.email)
        .execute(&self.pool)
        .await?;

        // Delete used token
        sqlx::query("DELETE FROM email_confirmation_tokens WHERE id = $1")
            .bind(token_record.id)
            .execute(&self.pool)
            .await?;

        // Log security event
        let _ = self.security_log.email_confirmed(user_id, ip).await;

        Ok(user_id)
    }

    /// Resend confirmation email (T023)
    ///
    /// Steps:
    /// 1. Find user by email
    /// 2. Check if already verified
    /// 3. Check cooldown (rate limiting)
    /// 4. Generate new token
    /// 5. Invalidate old tokens
    /// 6. Send new confirmation email
    pub async fn resend_confirmation(
        &self,
        email: &str,
        _ip: Option<IpAddr>,
    ) -> Result<(), AuthError> {
        // Find user by email
        let user = UserService::get_by_email(&self.pool, email)
            .await
            .map_err(|_| {
                // Don't reveal if email exists
                tracing::debug!(
                    email_domain = email.split('@').next_back(),
                    "Resend confirmation for unknown email"
                );
                AuthError::UserNotFound
            })?;

        // Check if already verified
        if user.email_verified {
            return Err(AuthError::AlreadyVerified);
        }

        // Check cooldown - get most recent token (raw query)
        let recent_token: Option<DateTime<Utc>> = sqlx::query_scalar(
            r#"
            SELECT created_at
            FROM email_confirmation_tokens
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(user.id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(created_at) = recent_token {
            let cooldown_expires =
                created_at + Duration::minutes(RESEND_CONFIRMATION_COOLDOWN_MINUTES);
            if cooldown_expires > Utc::now() {
                return Err(AuthError::TooManyRequests);
            }
        }

        // Invalidate all existing tokens for this user/email
        sqlx::query("DELETE FROM email_confirmation_tokens WHERE user_id = $1")
            .bind(user.id)
            .execute(&self.pool)
            .await?;

        // Generate new token
        let (raw_token, token_hash) = Self::generate_token();
        let expires_at = Utc::now() + Duration::hours(EMAIL_CONFIRMATION_EXPIRY_HOURS);

        // Store new token (raw query)
        sqlx::query(
            r#"
            INSERT INTO email_confirmation_tokens (user_id, email, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(user.id)
        .bind(email)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        // Send confirmation email
        self.email_service
            .send_confirmation(email, &raw_token)
            .await
            .map_err(|e| AuthError::Email(e.to_string()))?;

        tracing::info!(
            user_id = %user.id,
            "Resent confirmation email"
        );

        Ok(())
    }

    /// Login a user with email and password (T031)
    ///
    /// Steps:
    /// 1. Find user by email (or perform fake verification for timing attack prevention)
    /// 2. Check account lockout status
    /// 3. Verify password
    /// 4. Check email verification status
    /// 5. Check 2FA status (defer actual verification to separate call)
    /// 6. Reset failed attempts on success
    /// 7. Create session
    /// 8. Log security event
    pub async fn login(
        &self,
        email: &str,
        password: &str,
        ip: Option<IpAddr>,
        user_agent: Option<String>,
        device_info: Option<String>,
    ) -> Result<LoginResult, AuthError> {
        // Find user by email - use timing-safe lookup
        let user_result = UserService::get_by_email(&self.pool, email).await;

        // If user doesn't exist, perform fake verification to prevent timing attacks
        let user = match user_result {
            Ok(user) => user,
            Err(_) => {
                // Perform fake password verification for timing attack prevention
                self.password_service.fake_verify(password);

                let _ = self
                    .security_log
                    .login_failure(email, "user_not_found", ip)
                    .await;

                return Err(AuthError::InvalidCredentials);
            }
        };

        // Check lockout status
        if let Some(locked_until) = user.locked_until {
            if locked_until > Utc::now() {
                let _ = self.security_log.login_locked(user.id, ip, &locked_until.to_rfc3339()).await;
                return Err(AuthError::AccountLocked(locked_until.format("%Y-%m-%d %H:%M:%S UTC").to_string()));
            } else {
                // Lockout expired, reset failed attempts
                self.reset_failed_attempts(user.id).await?;
            }
        }

        // Verify password
        let is_valid = self
            .password_service
            .verify(password, &user.password_hash)
            .map_err(|e| AuthError::Password(e.to_string()))?;

        if !is_valid {
            // Increment failed attempts
            self.increment_failed_attempts(user.id, ip).await?;

            let _ = self
                .security_log
                .login_failure(email, "invalid_password", ip)
                .await;

            return Err(AuthError::InvalidCredentials);
        }

        // Check email verification
        if !user.email_verified {
            let _ = self
                .security_log
                .login_failure(email, "email_not_verified", ip)
                .await;
            return Err(AuthError::EmailNotVerified);
        }

        // Check if 2FA is enabled
        if user.totp_enabled {
            // Don't create session yet - require 2FA verification
            return Ok(LoginResult::TwoFactorRequired { user_id: user.id });
        }

        // Reset failed attempts on successful login
        self.reset_failed_attempts(user.id).await?;

        // Create session
        let session_service = self.session_service();
        let (session_id, token, expires_at) = session_service
            .create(user.id, ip, user_agent.clone(), device_info)
            .await
            .map_err(|e| AuthError::Session(e.to_string()))?;

        // Log successful login
        let _ = self
            .security_log
            .login_success(user.id, ip, user_agent)
            .await;

        Ok(LoginResult::Success {
            user_id: user.id,
            session_id,
            token,
            expires_at,
        })
    }

    /// Logout a user and terminate their session (T032)
    pub async fn logout(&self, user_id: Uuid, session_id: Uuid) -> Result<(), AuthError> {
        let session_service = self.session_service();

        // Revoke the session
        let revoked = session_service
            .revoke_by_id(session_id)
            .await
            .map_err(|e| AuthError::Session(e.to_string()))?;

        if !revoked {
            return Err(AuthError::InvalidToken);
        }

        // Log the logout
        let _ = self.security_log.logout(user_id, session_id).await;

        Ok(())
    }

    /// Check if an account is locked (T033)
    pub async fn check_lockout(&self, user_id: Uuid) -> Result<Option<DateTime<Utc>>, AuthError> {
        let locked_until: Option<DateTime<Utc>> = sqlx::query_scalar(
            "SELECT locked_until FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        // If lockout has expired, reset failed attempts
        if let Some(until) = locked_until {
            if until <= Utc::now() {
                self.reset_failed_attempts(user_id).await?;
                return Ok(None);
            }
        }

        Ok(locked_until)
    }

    /// Increment failed login attempts and potentially lock the account (T033)
    async fn increment_failed_attempts(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<(), AuthError> {
        // Increment failed attempts
        let new_count: i32 = sqlx::query_scalar(
            r#"
            UPDATE users
            SET failed_login_attempts = failed_login_attempts + 1, updated_at = NOW()
            WHERE id = $1
            RETURNING failed_login_attempts
            "#,
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        // Check if we need to lock the account
        if new_count >= MAX_FAILED_LOGIN_ATTEMPTS {
            let locked_until = Utc::now() + Duration::minutes(LOCKOUT_DURATION_MINUTES);

            sqlx::query(
                r#"
                UPDATE users
                SET locked_until = $2, updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(user_id)
            .bind(locked_until)
            .execute(&self.pool)
            .await?;

            // Log the lockout
            let _ = self
                .security_log
                .login_locked(user_id, ip, &locked_until.to_rfc3339())
                .await;

            tracing::warn!(
                user_id = %user_id,
                failed_attempts = new_count,
                locked_until = %locked_until,
                "Account locked due to failed login attempts"
            );
        }

        Ok(())
    }

    /// Reset failed login attempts (T033)
    async fn reset_failed_attempts(&self, user_id: Uuid) -> Result<(), AuthError> {
        sqlx::query(
            r#"
            UPDATE users
            SET failed_login_attempts = 0, locked_until = NULL, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create a session service instance
    pub fn session_service(&self) -> SessionService {
        SessionService::new(self.pool.clone(), self.session_expiry_days)
    }

    /// Get the security log service
    pub fn security_log(&self) -> &SecurityLogService {
        &self.security_log
    }

    /// Get the password service
    pub fn password_service(&self) -> &PasswordService {
        &self.password_service
    }
}

/// Result of a login attempt
#[derive(Debug)]
pub enum LoginResult {
    /// Login successful - session created
    Success {
        user_id: Uuid,
        session_id: Uuid,
        token: String,
        expires_at: DateTime<Utc>,
    },
    /// 2FA is required - user must provide TOTP code
    TwoFactorRequired {
        user_id: Uuid,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let (token, hash) = AuthService::generate_token();

        // Token should be 64 hex chars (32 bytes)
        assert_eq!(token.len(), 64);
        // Hash should be 64 hex chars (SHA-256 = 32 bytes)
        assert_eq!(hash.len(), 64);

        // Verify hash matches
        let computed_hash = AuthService::hash_token(&token).unwrap();
        assert_eq!(hash, computed_hash);
    }

    #[test]
    fn test_hash_token_invalid() {
        // Invalid hex should return error
        assert!(AuthService::hash_token("not-hex").is_err());
        assert!(AuthService::hash_token("xyz123").is_err());
    }

    #[test]
    fn test_reserved_usernames() {
        // Verify reserved usernames list exists and has expected entries
        assert!(RESERVED_USERNAMES.contains(&"admin"));
        assert!(RESERVED_USERNAMES.contains(&"api"));
        assert!(RESERVED_USERNAMES.contains(&"login"));
    }
}
