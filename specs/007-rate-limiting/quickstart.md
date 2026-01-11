# Quickstart: Rate Limiting

**Feature**: 007-rate-limiting
**Date**: 2026-01-10

## Prerequisites

- Rust 1.75+ installed
- PostgreSQL running with existing Oppskrift schema
- Existing Oppskrift codebase checked out

## Configuration

Set environment variables (or add to `.env`):

```bash
# General API limits (requests per minute)
RATE_LIMIT_API_AUTHENTICATED=100
RATE_LIMIT_API_UNAUTHENTICATED=30

# Auth endpoint limits
RATE_LIMIT_AUTH_FAILED=5           # per 15 minutes per IP
RATE_LIMIT_AUTH_ACCOUNT=10         # per hour per account

# Resource limits
RATE_LIMIT_EXPORT=1                # per hour
RATE_LIMIT_SEARCH=10               # per minute
RATE_LIMIT_UPLOAD=20               # per 5 minutes

# Trusted proxy configuration (for X-Forwarded-For)
TRUSTED_PROXIES=""                 # e.g., "10.0.0.0/8,172.16.0.0/12"
```

## Quick Test

1. Run migrations:
   ```bash
   sqlx migrate run
   ```

2. Start the server:
   ```bash
   cargo run
   ```

3. Test rate limiting:
   ```bash
   # Make 6 rapid requests to trigger limit (default: 5 per 15 min)
   for i in {1..6}; do
     curl -X POST http://localhost:3000/api/v1/auth/login \
       -H "Content-Type: application/json" \
       -d '{"email":"test@test.com","password":"wrong"}' \
       -w "\nStatus: %{http_code}\n"
   done
   ```

4. Expected output on 6th request:
   ```json
   {
     "error": "rate_limit_exceeded",
     "message": "Too many failed login attempts. Please wait 15 minutes before trying again.",
     "retry_after": 900
   }
   Status: 429
   ```

## Key Files

| File | Purpose |
|------|---------|
| `src/api/middleware/rate_limit.rs` | Rate limiting middleware |
| `src/core/config.rs` | Rate limit configuration |
| `tests/rate_limit_test.rs` | Integration tests |

## Verification Checklist

- [ ] 429 returned after exceeding limit
- [ ] Retry-After header present in response
- [ ] Security event logged in database
- [ ] Rate limit resets after window expires
- [ ] Authenticated users get higher limits
- [ ] Fail-open behavior on errors

## Common Issues

**Issue**: Rate limits not applied
**Solution**: Check middleware is added to router in `lib.rs`

**Issue**: All requests blocked (wrong IP)
**Solution**: Configure `TRUSTED_PROXIES` if behind load balancer

**Issue**: Rate limit state lost on restart
**Solution**: Expected behavior - in-memory storage. Distributed state is out of scope.
