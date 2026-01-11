# Implementation Plan: Rate Limiting

**Branch**: `007-rate-limiting` | **Date**: 2026-01-10 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/007-rate-limiting/spec.md`

## Summary

Implement comprehensive rate limiting to protect authentication endpoints from brute force attacks, prevent API abuse, and limit resource-intensive operations. Uses in-memory rate limiting via `tower_governor` middleware with configurable thresholds and proper 429 responses with Retry-After headers.

## Technical Context

**Language/Version**: Rust 1.75+ (2021 edition)
**Primary Dependencies**: Axum 0.8, tower_governor 0.8, tower 0.5
**Storage**: PostgreSQL (for security event logging), In-memory (for rate limit counters)
**Testing**: cargo test (integration tests in tests/)
**Target Platform**: Linux server (Scalingo deployment)
**Project Type**: Web application (server-rendered + API)
**Performance Goals**: <1ms rate limit check latency, 100 req/min authenticated, 30 req/min unauthenticated
**Constraints**: Single-instance deployment (in-memory state acceptable), fail-open on errors
**Scale/Scope**: Single server instance, ~1000 concurrent users

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Federation First | N/A | Rate limiting is local per-instance, does not affect federation protocol |
| II. Security by Default | ✅ PASS | Rate limiting enabled by default on all public endpoints (FR-012 configurable) |
| III. Standards Compliance | ✅ PASS | Uses HTTP 429 status code, Retry-After header per RFC 6585 |
| IV. User Experience | ✅ PASS | Clear error messages (FR-010), reasonable limits for shared IPs (SC-005) |
| V. Maintainability | ✅ PASS | Environment variable configuration (FR-012), existing middleware patterns |
| VI. Open Source Ethos | ✅ PASS | No proprietary services required |
| VII. Security Integration | ⚠️ PARTIAL | OSK integration deferred - existing security_events logging will be used |

**Gate Result**: PASS (VII partial is acceptable - logging infrastructure exists)

## Project Structure

### Documentation (this feature)

```text
specs/007-rate-limiting/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── api/
│   └── middleware/
│       ├── mod.rs           # Add rate_limit module export
│       ├── rate_limit.rs    # NEW: Rate limiting middleware
│       └── auth.rs          # Existing auth middleware
├── core/
│   ├── audit.rs             # Existing: Add rate_limit event type
│   ├── config.rs            # Existing: Add rate limit config
│   └── error.rs             # Existing: Add RateLimited error variant
├── lib.rs                   # Apply rate limit middleware to router
└── models/
    └── security_event.rs    # Existing: Add rate_limit_exceeded event type

tests/
├── rate_limit_test.rs       # NEW: Rate limiting integration tests
└── common/mod.rs            # Add rate limit test helpers
```

**Structure Decision**: Extends existing single-project structure. Rate limiting is implemented as middleware in `src/api/middleware/rate_limit.rs`, following the existing pattern of `auth.rs` and `security.rs`.

## Complexity Tracking

No constitution violations requiring justification.

## Phase Completion

### Phase 0: Research ✅

- **Output**: [research.md](./research.md)
- **Topics Resolved**:
  - tower_governor integration with Axum 0.8
  - Trusted proxy configuration for IP extraction
  - Rate limit tiers architecture (global, auth, resource)
  - 429 response format with Retry-After header
  - Fail-open behavior implementation
  - Security event logging integration
  - Environment variable configuration

### Phase 1: Design & Contracts ✅

- **Data Model**: [data-model.md](./data-model.md)
  - Rate limit configuration (in-memory)
  - Rate limit counters (governor state)
  - Security event logging (existing table, new event type)

- **API Contracts**: [contracts/rate-limit-responses.yaml](./contracts/rate-limit-responses.yaml)
  - 429 response schema
  - Rate limit headers (Retry-After, X-RateLimit-*)
  - Example responses for different limit types

- **Quickstart**: [quickstart.md](./quickstart.md)
  - Configuration guide
  - Quick test procedure
  - Verification checklist

### Phase 2: Tasks (Pending)

Run `/speckit.tasks` to generate implementation tasks.
