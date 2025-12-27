# Quickstart: User Authentication

**Feature**: `002-user-auth`
**Date**: 2025-12-27
**Phase**: Complete

## Prerequisites

- Rust 1.75+
- PostgreSQL 15+
- SMTP server for email delivery (optional in dev - emails are logged)
- Running instance from feature 001

## Environment Variables

Add to `.env`:

```bash
# Authentication (REQUIRED - no defaults)
JWT_SECRET=your-256-bit-secret-here-minimum-32-chars
TOTP_ENCRYPTION_KEY=64-hex-characters-for-32-bytes-aes-key

# S3/Storage (REQUIRED)
S3_BUCKET=your-bucket-name

# Email configuration (optional - emails logged if not set)
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USER=noreply@oppskrift.example.com
SMTP_PASSWORD=your-smtp-password
EMAIL_FROM_ADDRESS=noreply@oppskrift.example.com
EMAIL_FROM_NAME=Oppskrift

# Base URL for email links
BASE_URL=http://localhost:3000

# Session (optional, defaults shown)
SESSION_EXPIRY_DAYS=7
```

## Database Migration

Run migrations to add auth tables:

```bash
sqlx migrate run
```

## Quick Test

### 1. Register a User

```bash
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "username": "testuser",
    "password": "SecurePass123!",
    "display_name": "Test User"
  }'
```

Expected response:
```json
{
  "message": "Registration successful. Please check your email to confirm your account.",
  "user_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### 2. Confirm Email

Check server logs for confirmation link (in dev mode), or check your email:

```bash
# In development, the token is logged. Use it:
curl http://localhost:3000/api/v1/auth/confirm-email/{token}
```

Expected response:
```json
{
  "message": "Email confirmed successfully. You can now log in."
}
```

### 3. Login

```bash
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "SecurePass123!"
  }' \
  -c cookies.txt
```

Expected response (without 2FA):
```json
{
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "testuser",
    "email": "test@example.com",
    "display_name": "Test User"
  },
  "session_id": "660e8400-e29b-41d4-a716-446655440001",
  "expires_at": "2025-01-03T12:00:00Z"
}
```

### 4. Access Protected Endpoints

```bash
# Get profile
curl http://localhost:3000/api/v1/account/profile \
  -b cookies.txt

# Get security info
curl http://localhost:3000/api/v1/account/security \
  -b cookies.txt

# List active sessions
curl http://localhost:3000/api/v1/account/sessions \
  -b cookies.txt

# View security audit log
curl "http://localhost:3000/api/v1/account/security-events?limit=20" \
  -b cookies.txt
```

### 5. Change Password

```bash
curl -X POST http://localhost:3000/api/v1/account/change-password \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "SecurePass123!",
    "new_password": "NewSecurePass456!"
  }' \
  -b cookies.txt
```

Expected response:
```json
{
  "message": "Password changed successfully. Other sessions have been logged out.",
  "sessions_revoked": 2
}
```

### 6. Enable 2FA

```bash
# Step 1: Start setup (returns QR code)
curl -X POST http://localhost:3000/api/v1/account/2fa/setup \
  -b cookies.txt

# Response includes:
# - qr_code: Base64 PNG image
# - secret: Manual entry code
# - otpauth_uri: For authenticator apps

# Step 2: Scan QR with authenticator app, get 6-digit code

# Step 3: Enable 2FA
curl -X POST http://localhost:3000/api/v1/account/2fa/enable \
  -H "Content-Type: application/json" \
  -d '{"totp_code": "123456"}' \
  -b cookies.txt
```

Response includes recovery codes - **SAVE THESE!**
```json
{
  "message": "Two-factor authentication has been enabled.",
  "recovery_codes": [
    "ABCD-1234",
    "EFGH-5678",
    "..."
  ]
}
```

### 7. Login with 2FA

```bash
# Step 1: Initial login returns partial token
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "NewSecurePass456!"
  }'

# Response:
# {"requires_2fa": true, "partial_token": "abc123..."}

# Step 2: Complete login with TOTP code
curl -X POST http://localhost:3000/api/v1/auth/2fa/verify \
  -H "Content-Type: application/json" \
  -d '{
    "partial_token": "abc123...",
    "totp_code": "654321"
  }' \
  -c cookies.txt
```

### 8. Password Recovery

```bash
# Request password reset
curl -X POST http://localhost:3000/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com"}'

# Response is always the same (prevents email enumeration):
# {"message": "If an account exists with this email, a password reset link has been sent."}

# Reset password with token from email
curl -X POST http://localhost:3000/api/v1/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{
    "token": "reset-token-from-email",
    "new_password": "AnotherSecurePass789!"
  }'
```

### 9. Account Deletion

```bash
# Request deletion (7-day grace period)
curl -X POST http://localhost:3000/api/v1/account/delete \
  -H "Content-Type: application/json" \
  -d '{"password": "AnotherSecurePass789!"}' \
  -b cookies.txt

# Response:
# {
#   "message": "Account deletion scheduled...",
#   "scheduled_for": "2025-01-10T12:00:00Z",
#   "grace_period_days": 7
# }

# Cancel deletion (within grace period)
curl -X POST http://localhost:3000/api/v1/account/cancel-deletion \
  -b cookies.txt
```

### 10. Logout

```bash
curl -X POST http://localhost:3000/api/v1/auth/logout \
  -b cookies.txt
```

## Rate Limiting Test

```bash
# Trigger lockout (5 failed attempts)
for i in {1..6}; do
  curl -X POST http://localhost:3000/api/v1/auth/login \
    -H "Content-Type: application/json" \
    -d '{"email": "test@example.com", "password": "wrong"}'
  sleep 1
done

# Should return 403 with locked_until after 5 attempts
```

## Session Management

```bash
# List all sessions
curl http://localhost:3000/api/v1/account/sessions \
  -b cookies.txt

# Revoke a specific session (not the current one)
curl -X DELETE http://localhost:3000/api/v1/account/sessions/{session_id} \
  -b cookies.txt
```

## Security Checklist

Before deploying:

- [ ] JWT_SECRET is a cryptographically random 32+ character string
- [ ] TOTP_ENCRYPTION_KEY is a 64-character hex string (32 bytes)
- [ ] HTTPS is enabled (required for Secure cookies)
- [ ] SMTP is configured for email delivery
- [ ] Rate limiting is tested and working
- [ ] Session cookie flags are correct (HttpOnly, Secure, SameSite=Strict)
- [ ] Security audit logging is verified
- [ ] Cleanup job is scheduled (cron: `0 3 * * * /path/to/cleanup-job`)
- [ ] No development secrets in production

## Troubleshooting

### "Invalid credentials" on every login
- Check email is confirmed (email_verified = true)
- Check account is not locked (locked_until < NOW())
- Verify password meets requirements (10+ chars)

### Session not persisting
- Check cookie domain/path matches
- Verify HTTPS in production (Secure flag)
- Check expires_at is in future
- Ensure session token is included in requests

### 2FA codes always fail
- Check server time is synchronized (NTP)
- Verify authenticator app time is correct
- Allow for ~30 second window (1 step skew)
- Try a fresh code (codes change every 30s)

### Emails not sending
- Check SMTP credentials
- Verify firewall allows outbound SMTP (port 587)
- Check spam folder
- In development, check server logs for email content

### Account locked
- Wait for lockout period (15 minutes default)
- Or clear lockout in database:
  ```sql
  UPDATE users SET failed_login_attempts = 0, locked_until = NULL
  WHERE email = 'test@example.com';
  ```
