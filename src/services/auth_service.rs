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

/// 2FA pending token expiry in minutes (short-lived for security)
const TWO_FACTOR_PENDING_EXPIRY_MINUTES: i64 = 5;

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
    #[error("Invalid two-factor code")]
    InvalidTwoFactorCode,
    #[error("No local password set (federated user)")]
    NoLocalPassword,
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
        if user.email_verified && user.email.as_ref() == Some(&token_record.email) {
            // Delete the token since it's no longer needed
            sqlx::query("DELETE FROM email_confirmation_tokens WHERE id = $1")
                .bind(token_record.id)
                .execute(&self.pool)
                .await?;
            return Err(AuthError::AlreadyVerified);
        }

        // Check if this is an email change (not initial verification)
        let old_email = user.email.clone();
        let is_email_change =
            old_email.is_some() && old_email.as_ref() != Some(&token_record.email);

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

        // Log security events
        if is_email_change {
            // Safe to unwrap because is_email_change is only true when old_email.is_some()
            let old = old_email.as_deref().unwrap();
            let _ = self
                .security_log
                .email_change(user_id, old, &token_record.email, ip)
                .await;
        } else {
            let _ = self.security_log.email_confirmed(user_id, ip).await;
        }

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
                let _ = self
                    .security_log
                    .login_locked(user.id, ip, &locked_until.to_rfc3339())
                    .await;
                return Err(AuthError::AccountLocked(
                    locked_until.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                ));
            } else {
                // Lockout expired, reset failed attempts
                self.reset_failed_attempts(user.id).await?;
            }
        }

        // Verify password (federated users can't login locally)
        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or(AuthError::NoLocalPassword)?;

        let is_valid = self
            .password_service
            .verify(password, password_hash)
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
            // Create a pending 2FA token instead of session
            let partial_token = self
                .create_2fa_pending_token(user.id, ip, user_agent)
                .await?;
            return Ok(LoginResult::TwoFactorRequired { partial_token });
        }

        // Reset failed attempts on successful login
        self.reset_failed_attempts(user.id).await?;

        // Create session
        let session_service = self.session_service();
        let (_session_id, token, expires_at) = session_service
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

    // =========================================================================
    // 2FA Login Flow
    // =========================================================================

    /// Create a pending 2FA token for login completion
    async fn create_2fa_pending_token(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
        user_agent: Option<String>,
    ) -> Result<String, AuthError> {
        // Generate token
        let (raw_token, token_hash) = Self::generate_token();
        let expires_at = Utc::now() + Duration::minutes(TWO_FACTOR_PENDING_EXPIRY_MINUTES);

        // Delete any existing pending token for this user (only one allowed)
        sqlx::query("DELETE FROM two_factor_pending_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Store new pending token
        sqlx::query(
            r#"
            INSERT INTO two_factor_pending_tokens (user_id, token_hash, ip_address, user_agent, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(user_id)
        .bind(&token_hash)
        .bind(ip.map(|ip| ip.to_string()))
        .bind(&user_agent)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(raw_token)
    }

    /// Complete 2FA login by verifying TOTP code
    ///
    /// Takes the partial token from login and a TOTP code, verifies both,
    /// and creates a session on success.
    pub async fn complete_2fa_login(
        &self,
        partial_token: &str,
        totp_code: &str,
        ip: Option<IpAddr>,
        device_info: Option<String>,
    ) -> Result<LoginResult, AuthError> {
        // Hash the token for lookup
        let token_hash = Self::hash_token(partial_token)?;

        // Find and validate the pending token
        #[derive(sqlx::FromRow)]
        struct PendingToken {
            user_id: Uuid,
            user_agent: Option<String>,
            expires_at: DateTime<Utc>,
        }

        let pending: Option<PendingToken> = sqlx::query_as(
            r#"
            SELECT user_id, user_agent, expires_at
            FROM two_factor_pending_tokens
            WHERE token_hash = $1
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let pending = pending.ok_or(AuthError::InvalidToken)?;

        // Check expiry
        if pending.expires_at < Utc::now() {
            // Delete expired token
            sqlx::query("DELETE FROM two_factor_pending_tokens WHERE token_hash = $1")
                .bind(&token_hash)
                .execute(&self.pool)
                .await?;
            return Err(AuthError::InvalidToken);
        }

        // Verify TOTP code using TotpService
        let totp_service = crate::services::TotpService::from_env(self.pool.clone())
            .map_err(|e| AuthError::App(format!("TOTP service error: {}", e)))?;

        let is_valid = totp_service
            .verify_totp(pending.user_id, totp_code)
            .await
            .map_err(|e| match e {
                crate::services::TotpError::NotEnabled => {
                    AuthError::App("2FA is no longer enabled".to_string())
                }
                _ => AuthError::App(format!("TOTP verification error: {}", e)),
            })?;

        if !is_valid {
            let _ = self
                .security_log
                .two_factor_failure(pending.user_id, "invalid_totp", ip)
                .await;
            return Err(AuthError::InvalidTwoFactorCode);
        }

        // Delete the pending token
        sqlx::query("DELETE FROM two_factor_pending_tokens WHERE user_id = $1")
            .bind(pending.user_id)
            .execute(&self.pool)
            .await?;

        // Reset failed attempts
        self.reset_failed_attempts(pending.user_id).await?;

        // Create session
        let session_service = self.session_service();
        let (_session_id, token, expires_at) = session_service
            .create(pending.user_id, ip, pending.user_agent.clone(), device_info)
            .await
            .map_err(|e| AuthError::Session(e.to_string()))?;

        // Log successful login with 2FA
        let _ = self
            .security_log
            .login_success_2fa(pending.user_id, ip, pending.user_agent)
            .await;

        Ok(LoginResult::Success {
            user_id: pending.user_id,
            token,
            expires_at,
        })
    }

    /// Check if an account is locked (T033)
    pub async fn check_lockout(&self, user_id: Uuid) -> Result<Option<DateTime<Utc>>, AuthError> {
        let locked_until: Option<DateTime<Utc>> =
            sqlx::query_scalar("SELECT locked_until FROM users WHERE id = $1")
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

    /// Get the password service
    pub fn password_service(&self) -> &PasswordService {
        &self.password_service
    }

    // =========================================================================
    // Password Recovery (T043-T044)
    // =========================================================================

    /// Password reset token expiry in hours
    const PASSWORD_RESET_EXPIRY_HOURS: i64 = 1;

    /// Request a password reset (T043)
    ///
    /// Always returns success to prevent email enumeration.
    /// If user exists and email is verified, sends reset email.
    pub async fn forgot_password(&self, email: &str, ip: Option<IpAddr>) -> Result<(), AuthError> {
        // Look up user - don't reveal if exists
        let user_result = UserService::get_by_email(&self.pool, email).await;

        match user_result {
            Ok(user) => {
                // Only send if email is verified
                if !user.email_verified {
                    tracing::debug!(
                        email_domain = email.split('@').next_back(),
                        "Password reset requested for unverified email"
                    );
                    return Ok(()); // Silent success
                }

                // Generate reset token
                let (token, token_hash) = Self::generate_token();
                let expires_at = Utc::now() + Duration::hours(Self::PASSWORD_RESET_EXPIRY_HOURS);

                // Invalidate any existing tokens for this user
                sqlx::query("DELETE FROM password_reset_tokens WHERE user_id = $1")
                    .bind(user.id)
                    .execute(&self.pool)
                    .await?;

                // Store new token
                sqlx::query(
                    r#"
                    INSERT INTO password_reset_tokens (id, user_id, token_hash, created_at, expires_at)
                    VALUES ($1, $2, $3, NOW(), $4)
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(user.id)
                .bind(&token_hash)
                .bind(expires_at)
                .execute(&self.pool)
                .await?;

                // Send reset email (use the input email since we found user by it)
                if let Err(e) = self.email_service.send_password_reset(email, &token).await {
                    tracing::error!(error = %e, "Failed to send password reset email");
                    // Don't fail the request - token is saved
                }

                // Log security event
                let _ = self.security_log.password_reset_request(email, ip).await;

                tracing::info!(
                    user_id = %user.id,
                    "Password reset requested"
                );
            }
            Err(_) => {
                // User not found - silent success to prevent enumeration
                tracing::debug!(
                    email_domain = email.split('@').next_back(),
                    "Password reset requested for unknown email"
                );
            }
        }

        Ok(())
    }

    /// Reset password with token (T044)
    ///
    /// Validates token, sets new password, and invalidates all sessions.
    pub async fn reset_password(
        &self,
        token: &str,
        new_password: &str,
        ip: Option<IpAddr>,
    ) -> Result<(), AuthError> {
        // Hash the provided token
        let token_hash = Self::hash_token(token).map_err(|_| AuthError::InvalidToken)?;

        // Find valid token
        let reset_token: Option<(Uuid, Uuid, Option<DateTime<Utc>>)> = sqlx::query_as(
            r#"
            SELECT id, user_id, used_at
            FROM password_reset_tokens
            WHERE token_hash = $1 AND expires_at > NOW()
            "#,
        )
        .bind(&token_hash)
        .fetch_optional(&self.pool)
        .await?;

        let (token_id, user_id, used_at) = reset_token.ok_or(AuthError::InvalidToken)?;

        // Check if already used
        if used_at.is_some() {
            return Err(AuthError::InvalidToken);
        }

        // Validate new password strength
        self.password_service
            .validate_strength(new_password)
            .map_err(|e| AuthError::InvalidPassword(e.to_string()))?;

        // Hash new password
        let password_hash = self
            .password_service
            .hash(new_password)
            .map_err(|e| AuthError::Password(e.to_string()))?;

        // Update password
        sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(&password_hash)
        .execute(&self.pool)
        .await?;

        // Mark token as used
        sqlx::query(
            r#"
            UPDATE password_reset_tokens
            SET used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(token_id)
        .execute(&self.pool)
        .await?;

        // Invalidate all sessions for security
        let session_service = self.session_service();
        let _ = session_service.revoke_all_for_user(user_id).await;

        // Log security event
        let _ = self.security_log.password_reset_complete(user_id, ip).await;

        tracing::info!(
            user_id = %user_id,
            "Password reset completed"
        );

        Ok(())
    }

    // =========================================================================
    // Account Security (T053-T054)
    // =========================================================================

    /// Change password (T053)
    ///
    /// Verifies current password, sets new password, and invalidates other sessions.
    /// Returns the count of sessions that were revoked.
    pub async fn change_password(
        &self,
        user_id: Uuid,
        current_session_id: Uuid,
        current_password: &str,
        new_password: &str,
        ip: Option<IpAddr>,
    ) -> Result<u32, AuthError> {
        // Get user
        let user: User = UserService::get_by_id(&self.pool, user_id).await?;

        // Verify current password (federated users can't change local password)
        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or(AuthError::NoLocalPassword)?;

        let is_valid = self
            .password_service
            .verify(current_password, password_hash)
            .map_err(|e| AuthError::Password(e.to_string()))?;

        if !is_valid {
            let _ = self
                .security_log
                .password_change_failed(user_id, "invalid_current_password", ip)
                .await;
            return Err(AuthError::InvalidCredentials);
        }

        // Validate new password
        self.password_service
            .validate_new_password(new_password)
            .await
            .map_err(|e| AuthError::InvalidPassword(e.to_string()))?;

        // Hash new password
        let password_hash = self
            .password_service
            .hash(new_password)
            .map_err(|e| AuthError::Password(e.to_string()))?;

        // Update password
        sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(&password_hash)
        .execute(&self.pool)
        .await?;

        // Revoke all sessions except current
        let session_service = self.session_service();
        let sessions_revoked = session_service
            .revoke_others_for_user(user_id, current_session_id)
            .await
            .map_err(|e| AuthError::Session(e.to_string()))?;

        // Log security events
        let _ = self.security_log.password_change(user_id, ip).await;
        if sessions_revoked > 0 {
            let _ = self
                .security_log
                .session_revoke_all(user_id, sessions_revoked as u32, ip)
                .await;
        }

        // Send notification email (if user has email)
        if let Some(email) = &user.email {
            if let Err(e) = self
                .email_service
                .send_password_changed_notification(email)
                .await
            {
                tracing::warn!(error = %e, "Failed to send password change notification");
            }
        }

        tracing::info!(
            user_id = %user_id,
            sessions_revoked = sessions_revoked,
            "Password changed"
        );

        Ok(sessions_revoked as u32)
    }

    /// Change email address (T054)
    ///
    /// Generates a confirmation token for the new email.
    /// The old email remains active until the new one is confirmed.
    pub async fn change_email(
        &self,
        user_id: Uuid,
        new_email: &str,
        password: &str,
        ip: Option<IpAddr>,
    ) -> Result<(), AuthError> {
        // Get user
        let user: User = UserService::get_by_id(&self.pool, user_id).await?;

        // Verify password (federated users can't change email locally)
        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or(AuthError::NoLocalPassword)?;

        let is_valid = self
            .password_service
            .verify(password, password_hash)
            .map_err(|e| AuthError::Password(e.to_string()))?;

        if !is_valid {
            return Err(AuthError::InvalidCredentials);
        }

        // Check if new email is same as current
        if let Some(current_email) = &user.email {
            if current_email.to_lowercase() == new_email.to_lowercase() {
                return Err(AuthError::App("New email is same as current".to_string()));
            }
        }

        // Check if new email is available
        if !UserService::email_available(&self.pool, new_email).await? {
            return Err(AuthError::EmailExists);
        }

        // Invalidate any existing confirmation tokens for this user
        sqlx::query("DELETE FROM email_confirmation_tokens WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        // Generate confirmation token for new email
        let (token, token_hash) = Self::generate_token();
        let expires_at = Utc::now() + Duration::hours(EMAIL_CONFIRMATION_EXPIRY_HOURS);

        // Store token (with new email)
        sqlx::query(
            r#"
            INSERT INTO email_confirmation_tokens (user_id, email, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(user_id)
        .bind(new_email)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        // Send confirmation email to new address
        if let Err(e) = self
            .email_service
            .send_confirmation(new_email, &token)
            .await
        {
            tracing::error!(error = %e, "Failed to send email change confirmation");
            return Err(AuthError::Email(e.to_string()));
        }

        // Log security event
        let _ = self.security_log.email_change_requested(user_id, ip).await;

        tracing::info!(
            user_id = %user_id,
            "Email change requested"
        );

        Ok(())
    }

    // =========================================================================
    // Account Deletion (T073-T075)
    // =========================================================================

    /// Grace period in days before deletion is executed
    const DELETION_GRACE_PERIOD_DAYS: i64 = 7;

    /// Request account deletion (T073)
    ///
    /// Schedules account for deletion after a 7-day grace period.
    /// User can cancel during this period.
    pub async fn request_deletion(
        &self,
        user_id: Uuid,
        password: &str,
        ip: Option<IpAddr>,
    ) -> Result<DateTime<Utc>, AuthError> {
        // Get user
        let user: User = UserService::get_by_id(&self.pool, user_id).await?;

        // Verify password (federated users can't request local deletion)
        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or(AuthError::NoLocalPassword)?;

        let is_valid = self
            .password_service
            .verify(password, password_hash)
            .map_err(|e| AuthError::Password(e.to_string()))?;

        if !is_valid {
            return Err(AuthError::InvalidCredentials);
        }

        // Check if already scheduled for deletion
        if user.deletion_requested_at.is_some() {
            return Err(AuthError::App(
                "Account is already scheduled for deletion".to_string(),
            ));
        }

        // Schedule deletion
        let deletion_date = Utc::now() + Duration::days(Self::DELETION_GRACE_PERIOD_DAYS);

        sqlx::query(
            r#"
            UPDATE users
            SET deletion_requested_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        // Log security event
        let _ = self.security_log.account_delete_request(user_id, ip).await;

        // Send notification email (if user has email)
        if let Some(email) = &user.email {
            if let Err(e) = self
                .email_service
                .send_deletion_scheduled_notification(email, deletion_date)
                .await
            {
                tracing::warn!(error = %e, "Failed to send deletion notification");
            }
        }

        tracing::info!(
            user_id = %user_id,
            deletion_date = %deletion_date,
            "Account deletion requested"
        );

        Ok(deletion_date)
    }

    /// Cancel account deletion (T074)
    ///
    /// Cancels a scheduled deletion during the grace period.
    pub async fn cancel_deletion(
        &self,
        user_id: Uuid,
        ip: Option<IpAddr>,
    ) -> Result<(), AuthError> {
        // Get user
        let user: User = UserService::get_by_id(&self.pool, user_id).await?;

        // Check if deletion is scheduled
        if user.deletion_requested_at.is_none() {
            return Err(AuthError::App("No deletion is scheduled".to_string()));
        }

        // Cancel deletion
        sqlx::query(
            r#"
            UPDATE users
            SET deletion_requested_at = NULL, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        // Log security event
        let _ = self.security_log.account_delete_cancel(user_id, ip).await;

        // Send confirmation email (if user has email)
        if let Some(email) = &user.email {
            if let Err(e) = self
                .email_service
                .send_deletion_cancelled_notification(email)
                .await
            {
                tracing::warn!(error = %e, "Failed to send cancellation notification");
            }
        }

        tracing::info!(
            user_id = %user_id,
            "Account deletion cancelled"
        );

        Ok(())
    }
}

/// Result of a login attempt
#[derive(Debug)]
pub enum LoginResult {
    /// Login successful - session created
    Success {
        user_id: Uuid,
        token: String,
        expires_at: DateTime<Utc>,
    },
    /// 2FA is required - user must provide TOTP code with partial token
    TwoFactorRequired {
        /// Partial token to complete 2FA (expires in 5 minutes)
        partial_token: String,
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
