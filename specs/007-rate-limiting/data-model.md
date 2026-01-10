# Data Model: Rate Limiting

**Feature**: 007-rate-limiting
**Date**: 2026-01-10

## Entities

### 1. Rate Limit Configuration (In-Memory)

Rate limit rules are defined in code and configured via environment variables. No database persistence required.

| Field | Type | Description |
|-------|------|-------------|
| `key_type` | Enum | IP, User, or Composite (IP+User) |
| `limit` | u32 | Maximum requests in window |
| `window` | Duration | Time window for counting |
| `scope` | String | Endpoint pattern or category |

**Variants**:
- `ApiAuthenticated`: 100 req/min per user
- `ApiUnauthenticated`: 30 req/min per IP
- `AuthFailed`: 5 req/15min per IP
- `AuthAccount`: 10 req/hour per account (across IPs)
- `Export`: 1 req/hour per user
- `Search`: 10 req/min per user
- `Upload`: 20 req/5min per user

### 2. Rate Limit Counter (In-Memory)

Tracked by governor's internal state. Not persisted.

| Field | Type | Description |
|-------|------|-------------|
| `key` | String | IP address, user ID, or composite key |
| `count` | AtomicU32 | Current request count in window |
| `window_start` | Instant | When current window started |

### 3. Security Event (Database - Existing)

Extends existing `security_events` table with new event type.

**Table**: `security_events` (existing)

| Column | Type | Description |
|--------|------|-------------|
| `id` | UUID | Primary key |
| `user_id` | UUID | User who triggered limit (nullable for unauthenticated) |
| `event_type` | Enum | Add `rate_limit_exceeded` variant |
| `ip_address` | INET | Client IP address |
| `metadata` | JSONB | Additional context |
| `created_at` | TIMESTAMP | When event occurred |

**New Event Type**: `rate_limit_exceeded`

**Metadata Schema**:
```json
{
  "endpoint": "/api/v1/auth/login",
  "limit_type": "auth_failed",
  "retry_after": 900,
  "request_count": 6,
  "limit": 5,
  "window_seconds": 900
}
```

## State Transitions

### Rate Limit State Machine

```
┌─────────────┐     request      ┌─────────────┐
│   ALLOWED   │ ───────────────► │  CHECKING   │
└─────────────┘                  └─────────────┘
       ▲                               │
       │                    ┌──────────┴──────────┐
       │                    ▼                     ▼
       │              under limit            over limit
       │                    │                     │
       │                    ▼                     ▼
       │            ┌─────────────┐       ┌─────────────┐
       └────────────│  INCREMENT  │       │   BLOCKED   │
        window      └─────────────┘       └─────────────┘
        expires                                  │
                                                 │ log event
                                                 ▼
                                          ┌─────────────┐
                                          │  429 RESP   │
                                          └─────────────┘
```

## Relationships

```
┌─────────────────┐          ┌─────────────────┐
│  HTTP Request   │          │   Rate Limit    │
│                 │─────────►│   Middleware    │
│  - IP address   │          │                 │
│  - User ID      │          │  - Extract key  │
│  - Endpoint     │          │  - Check limit  │
└─────────────────┘          └─────────────────┘
                                      │
                    ┌─────────────────┼─────────────────┐
                    ▼                 ▼                 ▼
           ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
           │   Governor  │   │  Security   │   │   Error     │
           │   Counter   │   │   Events    │   │  Response   │
           │  (memory)   │   │ (postgres)  │   │   (HTTP)    │
           └─────────────┘   └─────────────┘   └─────────────┘
```

## Validation Rules

1. **IP Address**: Must be valid IPv4 or IPv6, extracted from trusted proxy chain
2. **User ID**: Must be valid UUID from authenticated session (for user-based limits)
3. **Endpoint Pattern**: Must match configured rate limit scope
4. **Window**: Must be positive duration, minimum 1 second
5. **Limit**: Must be positive integer, minimum 1

## Database Migration

```sql
-- Add rate_limit_exceeded to security_event_type enum
ALTER TYPE security_event_type ADD VALUE IF NOT EXISTS 'rate_limit_exceeded';
```

No new tables required. Rate limit counters are in-memory only.
