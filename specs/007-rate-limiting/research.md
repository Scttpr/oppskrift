# Research: Rate Limiting

**Feature**: 007-rate-limiting
**Date**: 2026-01-10

## Research Topics

### 1. tower_governor Integration with Axum 0.8

**Decision**: Use `tower_governor` 0.8 with custom key extractors for IP and user-based limiting

**Rationale**:
- Already in Cargo.toml as a dependency
- Provides `GovernorLayer` that integrates with Tower middleware stack
- Supports custom key extraction via `KeyExtractor` trait
- In-memory state using governor's internal atomic counters meets <1ms latency requirement
- Supports Retry-After header generation automatically

**Alternatives Considered**:
- `actix-limitation`: Not compatible with Axum
- Custom implementation: Unnecessary complexity, governor is battle-tested
- `tower-limit`: Lower-level, requires more boilerplate

**Key Implementation Details**:
```rust
// Key extractor for IP-based limiting
struct IpKeyExtractor;
impl KeyExtractor for IpKeyExtractor {
    type Key = IpAddr;
    fn extract(&self, req: &Request<Body>) -> Result<Self::Key, GovernorError> {
        // Extract from X-Forwarded-For (first hop from trusted proxy) or ConnectInfo
    }
}

// Key extractor for user-based limiting (requires auth)
struct UserKeyExtractor;
impl KeyExtractor for UserKeyExtractor {
    type Key = Uuid;
    fn extract(&self, req: &Request<Body>) -> Result<Self::Key, GovernorError> {
        // Extract user ID from session/JWT
    }
}
```

### 2. Trusted Proxy Configuration for IP Extraction

**Decision**: Use environment variable `TRUSTED_PROXIES` with comma-separated CIDR ranges

**Rationale**:
- Scalingo provides known proxy IPs that can be configured
- X-Forwarded-For header should only be trusted from known proxies
- Prevents IP spoofing attacks by ignoring untrusted headers
- Falls back to direct connection IP when no trusted proxy detected

**Alternatives Considered**:
- Trust all X-Forwarded-For: Security vulnerability (IP spoofing)
- Ignore all headers: Breaks behind load balancers
- Cloudflare-specific headers: Too vendor-specific

**Configuration Format**:
```bash
# Scalingo's proxy CIDR ranges
TRUSTED_PROXIES="10.0.0.0/8,172.16.0.0/12"
```

### 3. Rate Limit Tiers Architecture

**Decision**: Implement three middleware layers with different extractors and limits

**Rationale**:
- Different endpoints have different abuse profiles
- Authentication needs stricter limits (brute force prevention)
- General API needs moderate limits (abuse prevention)
- Resource-intensive operations need strict limits (DoS prevention)

**Architecture**:
```
Layer 1: Global IP-based (30 req/min unauthenticated, 100 req/min authenticated)
         Applied to: All /api/* routes

Layer 2: Auth-specific IP-based (5 failed attempts/15 min)
         Applied to: /api/v1/auth/login, /api/v1/auth/register, /api/v1/auth/reset-password

Layer 3: Per-user resource limits (varies by endpoint)
         Applied to: /api/v1/users/me/export (1/hour), /api/v1/search (10/min), uploads (20/5min)
```

**Alternatives Considered**:
- Single global limit: Too coarse, can't protect auth specifically
- Per-endpoint configuration: Too complex, many duplicate limits
- Database-backed limits: Too slow for <1ms requirement

### 4. 429 Response Format and Headers

**Decision**: Return JSON error body with Retry-After header

**Rationale**:
- RFC 6585 defines 429 status code
- Retry-After header tells clients when to retry (seconds or HTTP-date)
- JSON body provides user-friendly message for display
- Consistent with existing error response format

**Response Format**:
```http
HTTP/1.1 429 Too Many Requests
Retry-After: 60
Content-Type: application/json

{
  "error": "rate_limit_exceeded",
  "message": "Too many requests. Please wait 60 seconds before trying again.",
  "retry_after": 60
}
```

### 5. Fail-Open Behavior

**Decision**: Allow requests when rate limit state is unavailable

**Rationale**:
- FR-013 requires fail-open behavior
- In-memory state failure is unlikely but possible (OOM, corruption)
- Blocking all traffic on state failure is worse than allowing potential abuse
- Log errors for monitoring but don't block

**Implementation**:
```rust
match governor.check_key(&key) {
    Ok(_) => Ok(next.run(req).await),
    Err(GovernorError::TooManyRequests { .. }) => Err(RateLimitResponse::new(...)),
    Err(GovernorError::UnableToExtractKey) => {
        tracing::warn!("Rate limit state error, failing open");
        Ok(next.run(req).await)  // Fail open
    }
}
```

### 6. Security Event Logging

**Decision**: Add `rate_limit_exceeded` event type to existing security_events table

**Rationale**:
- Existing infrastructure already logs security events
- Consistent with FR-011 requirement
- Enables admin visibility (User Story 4)
- No new tables needed

**Event Data**:
```sql
INSERT INTO security_events (user_id, event_type, ip_address, metadata)
VALUES ($1, 'rate_limit_exceeded', $2, jsonb_build_object(
    'endpoint', $3,
    'limit_type', $4,
    'retry_after', $5
));
```

### 7. Configuration via Environment Variables

**Decision**: Use environment variables with sensible defaults

**Rationale**:
- FR-012 requires configuration without code changes
- Matches existing config pattern in `core/config.rs`
- Scalingo deployment can set via dashboard

**Variables**:
```bash
# General API limits
RATE_LIMIT_API_AUTHENTICATED=100      # requests per minute
RATE_LIMIT_API_UNAUTHENTICATED=30     # requests per minute per IP

# Auth endpoint limits
RATE_LIMIT_AUTH_FAILED=5              # failed attempts per 15 minutes per IP
RATE_LIMIT_AUTH_ACCOUNT=10            # failed attempts per hour per account

# Resource limits
RATE_LIMIT_EXPORT=1                   # exports per hour
RATE_LIMIT_SEARCH=10                  # searches per minute
RATE_LIMIT_UPLOAD=20                  # uploads per 5 minutes

# Proxy configuration
TRUSTED_PROXIES=""                    # comma-separated CIDR ranges
```

## Summary

All NEEDS CLARIFICATION items resolved. Implementation will use:
- `tower_governor` for in-memory rate limiting middleware
- Custom key extractors for IP and user-based limiting
- Trusted proxy configuration for secure IP extraction
- Three-tier rate limiting (global, auth, resource)
- JSON 429 responses with Retry-After header
- Existing security_events table for logging
- Environment variable configuration
