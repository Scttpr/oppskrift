# Quickstart: User Authentication

**Feature**: `002-user-auth`
**Date**: 2025-12-26
**Phase**: 1 (Design)

## Prerequisites

- Rust 1.75+
- PostgreSQL 15+
- SMTP server for email delivery
- Running instance from feature 001

## Environment Variables

Add to `.env`:

```bash
# Authentication (REQUIRED - no defaults)
JWT_SECRET=your-256-bit-secret-here-minimum-32-chars
TOTP_ENCRYPTION_KEY=another-256-bit-secret-for-totp

# Email configuration
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USER=noreply@oppskrift.example.com
SMTP_PASSWORD=your-smtp-password
EMAIL_FROM=Oppskrift <noreply@oppskrift.example.com>

# Rate limiting (optional, defaults shown)
RATE_LIMIT_LOGIN_PER_IP=10        # per minute
RATE_LIMIT_LOGIN_PER_ACCOUNT=5    # attempts before lockout
LOCKOUT_DURATION_MINUTES=15

# Session (optional, defaults shown)
SESSION_EXPIRY_DAYS=7

# HIBP API (optional, for breach checking)
HIBP_USER_AGENT=Oppskrift/1.0
```

## Database Migration

Run migrations to add auth tables:

```bash
sqlx migrate run
```

Or manually:

```sql
-- Add auth fields to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS email VARCHAR(255) UNIQUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verified BOOLEAN DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_secret_encrypted BYTEA;
ALTER TABLE users ADD COLUMN IF NOT EXISTS totp_enabled BOOLEAN DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS failed_login_attempts INTEGER DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS locked_until TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS deletion_requested_at TIMESTAMPTZ;

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL,
    device_info VARCHAR(255),
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_activity TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

-- See data-model.md for complete schema
```

## Dependencies

Add to `Cargo.toml`:

```toml
# Password hashing
argon2 = "0.5"

# 2FA
totp-rs = { version = "5", features = ["qr", "otpauth", "serde_support"] }

# Email
lettre = { version = "0.11", features = ["tokio1-native-tls"] }

# Encryption
aes-gcm = "0.10"
sha1 = "0.10"
hex = "0.4"

# Already present: tower_governor, jsonwebtoken, uuid, chrono, validator
```

## Quick Test

### 1. Register a User

```bash
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "username": "testuser",
    "password": "SecurePass123",
    "display_name": "Test User"
  }'
```

Expected response:
```json
{
  "message": "Registration successful. Please check your email to confirm your account.",
  "user_id": "uuid-here"
}
```

### 2. Confirm Email

Check email for confirmation link, or in development:

```bash
# Get token from database
psql -c "SELECT token_hash FROM email_confirmation_tokens WHERE email='test@example.com';"

# Confirm (use the actual token from email link)
curl http://localhost:3000/api/auth/confirm-email/{token}
```

### 3. Login

```bash
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "SecurePass123"
  }' \
  -c cookies.txt
```

Expected response:
```json
{
  "user": {
    "id": "uuid",
    "username": "testuser",
    "email": "test@example.com",
    "display_name": "Test User",
    "totp_enabled": false
  },
  "session_token": "hex-token",
  "expires_at": "2025-01-02T...",
  "requires_2fa": false
}
```

### 4. Access Protected Endpoint

```bash
curl http://localhost:3000/api/account/profile \
  -b cookies.txt
```

### 5. Enable 2FA

```bash
# Start setup
curl -X POST http://localhost:3000/api/account/2fa/setup \
  -b cookies.txt

# Returns QR code and secret
# Scan QR with authenticator app
# Get 6-digit code

# Enable 2FA
curl -X POST http://localhost:3000/api/account/2fa/enable \
  -H "Content-Type: application/json" \
  -d '{"code": "123456"}' \
  -b cookies.txt

# Returns recovery codes - SAVE THESE!
```

### 6. Login with 2FA

```bash
# First request returns requires_2fa: true
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "SecurePass123"
  }'

# Second request with TOTP code
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "SecurePass123",
    "totp_code": "123456"
  }' \
  -c cookies.txt
```

## Rate Limiting Test

```bash
# Trigger lockout (5 failed attempts)
for i in {1..6}; do
  curl -X POST http://localhost:3000/api/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email": "test@example.com", "password": "wrong"}'
  sleep 1
done

# Should return 403 with locked_until
```

## Security Checklist

Before deploying:

- [ ] JWT_SECRET is set (not using fallback)
- [ ] TOTP_ENCRYPTION_KEY is set
- [ ] HTTPS is enabled
- [ ] SMTP is configured for email delivery
- [ ] Rate limiting is tested
- [ ] Session cookie flags are correct (HttpOnly, Secure, SameSite)
- [ ] Audit logging is enabled
- [ ] Expired sessions cleanup job is scheduled

## Troubleshooting

### "Invalid credentials" on every login
- Check email is confirmed
- Check account is not locked
- Verify password hash algorithm matches

### Session not persisting
- Check cookie domain/path
- Verify HTTPS in production (Secure flag)
- Check expires_at is not in past

### 2FA codes always fail
- Check server time is synchronized (NTP)
- Verify SHA1 algorithm (not SHA256/512)
- Allow time skew of 1 period

### Emails not sending
- Check SMTP credentials
- Verify firewall allows outbound SMTP
- Check spam folder
- Review logs for delivery errors
