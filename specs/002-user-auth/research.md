# Research: User Authentication

**Feature**: `002-user-auth`
**Date**: 2025-12-26
**Phase**: 0 (Research)

## Summary

This document captures technology decisions and research findings for implementing rock-solid user authentication in Oppskrift. The implementation will use proven Rust crates with security-first defaults.

## Technology Decisions

### Password Hashing: `argon2` (RustCrypto)

**Selected**: `argon2` crate from RustCrypto project

**Rationale**:
- Pure Rust implementation with OWASP-recommended defaults
- Supports Argon2id (winner of Password Hashing Competition)
- Over 12M downloads, actively maintained
- Memory-hard algorithm resistant to GPU/ASIC attacks
- Default params match OWASP recommendation: m=19456 (19 MiB), t=2, p=1

**Alternative considered**: `password-auth` - simpler API but less control over parameters

**Configuration**:
```rust
// OWASP recommended Argon2id parameters
let params = Params::new(19456, 2, 1, None)?; // 19 MiB, 2 iterations, 1 parallelism
let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
```

**Sources**:
- [RustCrypto password-hashes](https://github.com/RustCrypto/password-hashes)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)

---

### TOTP 2FA: `totp-rs`

**Selected**: `totp-rs` v5.x with features: `qr`, `otpauth`, `serde_support`

**Rationale**:
- RFC 6238 compliant implementation
- QR code generation for authenticator app setup
- otpauth URL parsing for standard interoperability
- Active maintenance, 1M+ downloads

**Features needed**:
- `qr` - Generate QR code for authenticator apps
- `otpauth` - Parse/generate otpauth:// URLs
- `serde_support` - Serialize TOTP config

**Important note**: SHA256/SHA512 algorithms may silently fallback to SHA1 in some authenticator apps. Recommend using SHA1 for maximum compatibility.

**Sources**:
- [totp-rs crates.io](https://crates.io/crates/totp-rs)
- [Rust 2FA Tutorial](https://codevoweb.com/rust-implement-2fa-two-factor-authentication/)

---

### Rate Limiting: `tower_governor`

**Selected**: `tower_governor` v0.4 (already in Cargo.toml)

**Rationale**:
- Tower middleware compatible with Axum
- Uses GCRA (Generic Cell Rate Algorithm) - sophisticated leaky bucket
- Supports per-IP, per-key, and global limiting
- Backed by `governor` crate, well-tested

**Key extractors needed**:
- `SmartIpKeyExtractor` - Uses x-forwarded-for, x-real-ip headers, falls back to peer IP
- Custom key extractor for per-account limiting (user_id + IP combination)

**Configuration for auth endpoints**:
```rust
// Per-IP: 10 requests/minute for login
// Per-account: 5 failed attempts, then 15-minute lockout
GovernorConfigBuilder::default()
    .per_second(1) // burst of 10 per minute
    .burst_size(10)
```

**Sources**:
- [tower_governor crates.io](https://crates.io/crates/tower_governor)
- [Tower middleware with Axum](https://medium.com/@khalludi123/creating-a-rate-limiter-middleware-using-tower-for-axum-rust-be1d65fbeca)

---

### Session Management: Custom with `tower-sessions` consideration

**Decision**: Custom implementation using database-backed sessions

**Rationale**:
- Full control over session lifecycle for security requirements
- Direct integration with user table for invalidation on password change
- Session metadata tracking (device, IP, last_activity) per FR-022

**Storage**:
- PostgreSQL table `sessions` with:
  - `id` (UUID, primary key)
  - `user_id` (FK to users)
  - `token_hash` (bcrypt hash of session token)
  - `device_info`, `ip_address`, `user_agent`
  - `created_at`, `last_activity`, `expires_at`

**Token format**: 256-bit random, hex-encoded (64 characters)

---

### Breached Password Check: HaveIBeenPwned API (k-anonymity)

**Decision**: Integrate HIBP Pwned Passwords API v3

**Rationale**:
- FR-004 requirement: Check passwords against breached list
- k-anonymity model: Only first 5 chars of SHA-1 hash sent to API
- No plaintext passwords leave the server
- Free API, no registration required

**Implementation**:
```rust
// 1. SHA-1 hash the password
// 2. Send first 5 characters to HIBP API
// 3. Search returned suffixes for match
// 4. If found, reject password with user-friendly message
```

**Offline alternative**: Download HIBP database (30GB+), impractical for this project

**Sources**:
- [HIBP Pwned Passwords API](https://haveibeenpwned.com/API/v3#PwnedPasswords)

---

### Email: `lettre` crate

**Selected**: `lettre` for SMTP email delivery

**Rationale**:
- De facto standard Rust email crate
- Async support with tokio
- TLS support built-in
- Template integration possible

**Usage**:
- Email confirmation links
- Password reset links
- Security notifications (password changed, 2FA enabled, new login)

---

### Encryption at Rest (TOTP Secrets): `aes-gcm`

**Selected**: `aes-gcm` from RustCrypto for encrypting TOTP secrets

**Rationale**:
- RISK-AUTH-007 requires TOTP secrets encrypted at rest
- AES-256-GCM provides authenticated encryption
- Key from environment variable (same pattern as JWT_SECRET)

---

## Dependencies to Add

```toml
# Authentication
argon2 = "0.5"                           # Password hashing
totp-rs = { version = "5", features = ["qr", "otpauth", "serde_support"] }  # 2FA
lettre = { version = "0.11", features = ["tokio1-native-tls"] }  # Email

# Encryption
aes-gcm = "0.10"                         # TOTP secret encryption
sha1 = "0.10"                            # HIBP k-anonymity hash

# Session management (additional)
hex = "0.4"                              # Token encoding
```

## Existing Dependencies (reuse)

- `tower_governor` - Rate limiting (already present)
- `jsonwebtoken` - JWT handling for API tokens (already present)
- `uuid` - Session and token IDs
- `chrono` - Timestamps and expiration
- `validator` - Input validation
- `reqwest` - HTTP client for HIBP API

---

## Database Schema Additions

### New Tables Required

```sql
-- User credentials (extends existing users table or new table)
ALTER TABLE users ADD COLUMN IF NOT EXISTS
    email VARCHAR(255) UNIQUE NOT NULL,
    email_verified BOOLEAN DEFAULT FALSE,
    password_hash VARCHAR(255) NOT NULL,
    totp_secret_encrypted BYTEA,  -- AES-256-GCM encrypted
    totp_enabled BOOLEAN DEFAULT FALSE,
    failed_login_attempts INTEGER DEFAULT 0,
    locked_until TIMESTAMPTZ,
    deletion_requested_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW();

-- Sessions
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,  -- bcrypt hash
    device_info VARCHAR(255),
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_activity TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

-- Password reset tokens
CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ
);

-- Email confirmation tokens
CREATE TABLE email_confirmation_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    token_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

-- 2FA Recovery codes
CREATE TABLE recovery_codes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash VARCHAR(255) NOT NULL,  -- bcrypt hash
    used_at TIMESTAMPTZ
);

-- Security events (audit log)
CREATE TABLE security_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    event_type VARCHAR(50) NOT NULL,  -- login, logout, password_change, etc.
    ip_address INET,
    user_agent TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
CREATE INDEX idx_security_events_user_id ON security_events(user_id);
CREATE INDEX idx_security_events_created_at ON security_events(created_at);
CREATE INDEX idx_password_reset_tokens_user_id ON password_reset_tokens(user_id);
```

---

## API Endpoints Design

### Public (No auth required)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/auth/register` | Create new account |
| POST | `/api/auth/login` | Authenticate user |
| POST | `/api/auth/logout` | Terminate session |
| POST | `/api/auth/forgot-password` | Request password reset |
| POST | `/api/auth/reset-password` | Set new password with token |
| GET | `/api/auth/confirm-email/:token` | Confirm email address |
| POST | `/api/auth/resend-confirmation` | Resend confirmation email |

### Protected (Auth required)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/account/profile` | Get current user profile |
| PATCH | `/api/account/profile` | Update profile |
| POST | `/api/account/change-password` | Change password |
| POST | `/api/account/change-email` | Request email change |
| GET | `/api/account/sessions` | List active sessions |
| DELETE | `/api/account/sessions/:id` | Revoke session |
| POST | `/api/account/2fa/setup` | Start 2FA setup (get QR) |
| POST | `/api/account/2fa/enable` | Verify and enable 2FA |
| POST | `/api/account/2fa/disable` | Disable 2FA |
| GET | `/api/account/2fa/recovery-codes` | Get recovery codes |
| POST | `/api/account/2fa/regenerate-recovery` | Regenerate recovery codes |
| POST | `/api/account/delete` | Request account deletion |
| POST | `/api/account/cancel-deletion` | Cancel deletion request |

---

## Security Controls Summary

| Risk ID | Control | Implementation |
|---------|---------|----------------|
| AUTH-001 | Rate limiting | tower_governor per IP + per account |
| AUTH-001 | Breach check | HIBP API k-anonymity |
| AUTH-002 | Password hashing | Argon2id OWASP params |
| AUTH-003 | Session security | HttpOnly, Secure, SameSite=Strict |
| AUTH-004 | Lockout | Progressive: 15m -> 30m -> 1h |
| AUTH-005 | Audit logging | security_events table |
| AUTH-006 | Reset tokens | 256-bit random, 1h expiry, single-use |
| AUTH-007 | TOTP encryption | AES-256-GCM at rest |
| AUTH-008 | No enumeration | Constant-time, generic errors |
| AUTH-009 | Session management | Full CRUD, metadata tracking |
| AUTH-010 | Timing attacks | Fake hash on unknown user |
| AUTH-011 | Session fixation | Regenerate on login |
| AUTH-012 | Recovery codes | Bcrypt hashed, single-use |

---

## Next Steps

1. **Phase 1**: Generate `data-model.md` with Rust structs
2. **Phase 1**: Generate `contracts/` with OpenAPI specs
3. **Phase 1**: Generate `quickstart.md` with setup guide
4. Complete `plan.md` with full technical context
