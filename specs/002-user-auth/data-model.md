# Data Model: User Authentication

**Feature**: `002-user-auth`
**Date**: 2025-12-26
**Phase**: 1 (Design)

## Entity Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                              users                                   │
├─────────────────────────────────────────────────────────────────────┤
│ id              UUID PK                                              │
│ username        VARCHAR(30) UNIQUE                                   │
│ email           VARCHAR(255) UNIQUE                                  │
│ email_verified  BOOLEAN                                              │
│ password_hash   VARCHAR(255)                                         │
│ display_name    VARCHAR(100)                                         │
│ bio             TEXT                                                 │
│ avatar_url      VARCHAR(500)                                         │
│ totp_secret_encrypted  BYTEA                                         │
│ totp_enabled    BOOLEAN                                              │
│ failed_login_attempts  INTEGER                                       │
│ locked_until    TIMESTAMPTZ                                          │
│ deletion_requested_at  TIMESTAMPTZ                                   │
│ created_at      TIMESTAMPTZ                                          │
│ updated_at      TIMESTAMPTZ                                          │
│ ap_id           VARCHAR(500)                                         │
│ federation_enabled  BOOLEAN                                          │
│ measurement_pref  measurement_pref                                   │
└─────────────────────────────────────────────────────────────────────┘
        │
        │ 1:N
        ▼
┌─────────────────────────────────────────────────────────────────────┐
│                            sessions                                  │
├─────────────────────────────────────────────────────────────────────┤
│ id              UUID PK                                              │
│ user_id         UUID FK → users.id ON DELETE CASCADE                 │
│ token_hash      VARCHAR(255)                                         │
│ device_info     VARCHAR(255)                                         │
│ ip_address      INET                                                 │
│ user_agent      TEXT                                                 │
│ created_at      TIMESTAMPTZ                                          │
│ last_activity   TIMESTAMPTZ                                          │
│ expires_at      TIMESTAMPTZ                                          │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                      password_reset_tokens                           │
├─────────────────────────────────────────────────────────────────────┤
│ id              UUID PK                                              │
│ user_id         UUID FK → users.id ON DELETE CASCADE                 │
│ token_hash      VARCHAR(255)                                         │
│ created_at      TIMESTAMPTZ                                          │
│ expires_at      TIMESTAMPTZ                                          │
│ used_at         TIMESTAMPTZ                                          │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                    email_confirmation_tokens                         │
├─────────────────────────────────────────────────────────────────────┤
│ id              UUID PK                                              │
│ user_id         UUID FK → users.id ON DELETE CASCADE                 │
│ email           VARCHAR(255)                                         │
│ token_hash      VARCHAR(255)                                         │
│ created_at      TIMESTAMPTZ                                          │
│ expires_at      TIMESTAMPTZ                                          │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                         recovery_codes                               │
├─────────────────────────────────────────────────────────────────────┤
│ id              UUID PK                                              │
│ user_id         UUID FK → users.id ON DELETE CASCADE                 │
│ code_hash       VARCHAR(255)                                         │
│ used_at         TIMESTAMPTZ                                          │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                        security_events                               │
├─────────────────────────────────────────────────────────────────────┤
│ id              UUID PK                                              │
│ user_id         UUID FK → users.id ON DELETE SET NULL                │
│ event_type      VARCHAR(50)                                          │
│ ip_address      INET                                                 │
│ user_agent      TEXT                                                 │
│ metadata        JSONB                                                │
│ created_at      TIMESTAMPTZ                                          │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Rust Structs

### User Entity (Extended)

```rust
// src/models/user.rs - Extensions for authentication

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// User entity with authentication fields
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    #[serde(skip_serializing)]  // Never expose hash
    pub password_hash: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub measurement_pref: MeasurementPref,
    #[serde(skip_serializing)]
    pub totp_secret_encrypted: Option<Vec<u8>>,
    pub totp_enabled: bool,
    #[serde(skip_serializing)]
    pub failed_login_attempts: i32,
    #[serde(skip_serializing)]
    pub locked_until: Option<DateTime<Utc>>,
    pub deletion_requested_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ap_id: String,
    pub federation_enabled: bool,
}

/// Reserved usernames that cannot be registered
pub const RESERVED_USERNAMES: &[&str] = &[
    "admin", "root", "system", "support", "help", "oppskrift",
    "api", "auth", "login", "logout", "register", "settings",
    "account", "profile", "user", "users", "mod", "moderator",
];
```

### Registration DTOs

```rust
// src/models/auth.rs

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Registration request
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(
        length(min = 3, max = 30, message = "Username must be 3-30 characters"),
        regex(path = "USERNAME_REGEX", message = "Username can only contain a-z, 0-9, and _")
    )]
    pub username: String,

    #[validate(length(min = 10, message = "Password must be at least 10 characters"))]
    pub password: String,

    #[validate(length(min = 1, max = 100, message = "Display name must be 1-100 characters"))]
    pub display_name: Option<String>,
}

lazy_static::lazy_static! {
    static ref USERNAME_REGEX: regex::Regex = regex::Regex::new(r"^[a-z0-9_]+$").unwrap();
}

/// Registration response
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub message: String,
    pub user_id: Uuid,
}
```

### Login DTOs

```rust
/// Login request
#[derive(Debug, Clone, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
    pub totp_code: Option<String>,  // Required if 2FA enabled
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserProfile,
    pub session_token: String,
    pub expires_at: DateTime<Utc>,
    pub requires_2fa: bool,  // If true, must call /login again with totp_code
}

/// 2FA required response (intermediate state)
#[derive(Debug, Serialize)]
pub struct TwoFactorRequired {
    pub message: String,
    pub session_token: String,  // Partial session, only valid for 2FA completion
}
```

### Session Entity

```rust
// src/models/session.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::net::IpAddr;
use uuid::Uuid;

/// Active session record
#[derive(Debug, Clone, FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub device_info: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Session info for display to user
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub id: Uuid,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub last_activity: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

/// Create new session
#[derive(Debug)]
pub struct CreateSession {
    pub user_id: Uuid,
    pub token_hash: String,
    pub device_info: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub expires_at: DateTime<Utc>,
}
```

### Password Reset Tokens

```rust
// src/models/password_reset.rs

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Password reset token record
#[derive(Debug, Clone, FromRow)]
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// Request password reset
#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email)]
    pub email: String,
}

/// Reset password with token
#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[validate(length(min = 10))]
    pub new_password: String,
}
```

### Email Confirmation Tokens

```rust
// src/models/email_confirmation.rs

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Email confirmation token record
#[derive(Debug, Clone, FromRow)]
pub struct EmailConfirmationToken {
    pub id: Uuid,
    pub user_id: Option<Uuid>,  // Can be null for new registrations
    pub email: String,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

### Recovery Codes (2FA)

```rust
// src/models/recovery_code.rs

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// 2FA recovery code record
#[derive(Debug, Clone, FromRow)]
pub struct RecoveryCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub used_at: Option<DateTime<Utc>>,
}

/// Recovery codes response (only shown once at generation)
#[derive(Debug, Serialize)]
pub struct RecoveryCodesResponse {
    pub codes: Vec<String>,  // Plaintext, show once
    pub message: String,
}
```

### Security Events (Audit Log)

```rust
// src/models/security_event.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::net::IpAddr;
use uuid::Uuid;

/// Security event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "snake_case")]
pub enum SecurityEventType {
    LoginSuccess,
    LoginFailed,
    LoginLocked,
    Logout,
    PasswordChange,
    PasswordResetRequest,
    PasswordResetComplete,
    EmailConfirmed,
    EmailChangeRequest,
    TwoFactorEnabled,
    TwoFactorDisabled,
    RecoveryCodeUsed,
    SessionRevoked,
    AccountDeletionRequest,
    AccountDeletionCancelled,
    AccountDeleted,
}

/// Security event record
#[derive(Debug, Clone, FromRow)]
pub struct SecurityEvent {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub event_type: String,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Create security event
#[derive(Debug)]
pub struct CreateSecurityEvent {
    pub user_id: Option<Uuid>,
    pub event_type: SecurityEventType,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
```

### 2FA DTOs

```rust
// src/models/two_factor.rs

use serde::{Deserialize, Serialize};
use validator::Validate;

/// 2FA setup response (start)
#[derive(Debug, Serialize)]
pub struct TwoFactorSetupResponse {
    pub secret: String,  // Base32 encoded
    pub qr_code: String,  // Base64 PNG
    pub otpauth_url: String,
}

/// Enable 2FA request (verify code)
#[derive(Debug, Deserialize, Validate)]
pub struct EnableTwoFactorRequest {
    #[validate(length(equal = 6))]
    pub code: String,
}

/// Disable 2FA request
#[derive(Debug, Deserialize, Validate)]
pub struct DisableTwoFactorRequest {
    pub password: String,
    #[validate(length(equal = 6))]
    pub code: String,
}
```

### Account Management DTOs

```rust
// src/models/account.rs

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Change password request
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    #[validate(length(min = 10))]
    pub new_password: String,
}

/// Change email request
#[derive(Debug, Deserialize, Validate)]
pub struct ChangeEmailRequest {
    pub password: String,
    #[validate(email)]
    pub new_email: String,
}

/// Request account deletion
#[derive(Debug, Deserialize)]
pub struct DeleteAccountRequest {
    pub password: String,
    pub delete_recipes: bool,  // true = delete, false = anonymize
}
```

---

## Password Validation Rules

```rust
// src/services/password.rs

/// Password requirements per FR-003
#[derive(Debug, Default)]
pub struct PasswordStrength {
    pub has_min_length: bool,      // >= 10 chars
    pub has_uppercase: bool,       // at least one A-Z
    pub has_lowercase: bool,       // at least one a-z
    pub has_number: bool,          // at least one 0-9
    pub is_not_breached: bool,     // not in HIBP database
}

impl PasswordStrength {
    pub fn is_valid(&self) -> bool {
        self.has_min_length
            && self.has_uppercase
            && self.has_lowercase
            && self.has_number
            && self.is_not_breached
    }

    pub fn missing_requirements(&self) -> Vec<&'static str> {
        let mut missing = vec![];
        if !self.has_min_length { missing.push("at least 10 characters"); }
        if !self.has_uppercase { missing.push("at least one uppercase letter"); }
        if !self.has_lowercase { missing.push("at least one lowercase letter"); }
        if !self.has_number { missing.push("at least one number"); }
        if !self.is_not_breached { missing.push("password appears in known data breaches"); }
        missing
    }
}
```

---

## Indexes

```sql
-- Performance indexes
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX idx_sessions_token_hash ON sessions(token_hash);  -- For lookups
CREATE INDEX idx_password_reset_tokens_user_id ON password_reset_tokens(user_id);
CREATE INDEX idx_password_reset_tokens_expires_at ON password_reset_tokens(expires_at);
CREATE INDEX idx_email_confirmation_tokens_email ON email_confirmation_tokens(email);
CREATE INDEX idx_recovery_codes_user_id ON recovery_codes(user_id);
CREATE INDEX idx_security_events_user_id ON security_events(user_id);
CREATE INDEX idx_security_events_created_at ON security_events(created_at);
CREATE INDEX idx_security_events_type ON security_events(event_type);
```

---

## Data Retention

| Table | Retention | Cleanup Strategy |
|-------|-----------|-----------------|
| sessions | expires_at + 7 days | Cron job daily |
| password_reset_tokens | used_at + 24h OR expires_at + 24h | Cron job daily |
| email_confirmation_tokens | expires_at + 7 days | Cron job daily |
| recovery_codes | Until regenerated | On regeneration |
| security_events | 90 days | Cron job weekly |
| users (deleted) | deletion_requested_at + 7 days | Grace period then hard delete |
