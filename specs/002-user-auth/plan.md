# Implementation Plan: User Authentication

**Branch**: `002-user-auth` | **Date**: 2025-12-26 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-user-auth/spec.md`

## Summary

Implement a rock-solid, user-friendly authentication system for Oppskrift with:
- Email/password registration with confirmation
- Secure session management (7-day expiry, multi-device)
- Password recovery with 1-hour expiring tokens
- TOTP-based two-factor authentication (P3)
- Account deletion with 7-day grace period
- Full security audit logging

Technical approach: Argon2id password hashing, tower_governor rate limiting, database-backed sessions, AES-256-GCM encrypted TOTP secrets.

## Technical Context

**Language/Version**: Rust 1.75+
**Primary Dependencies**: Axum 0.8, SQLx 0.8, argon2, totp-rs, lettre, tower_governor
**Storage**: PostgreSQL 15+ (sessions, tokens, security_events tables)
**Testing**: cargo test (unit + integration)
**Target Platform**: Linux server (Docker/Podman)
**Project Type**: Single (backend API + SSR templates)
**Performance Goals**: 1000 concurrent auth requests without degradation (SC-010)
**Constraints**: <200ms p95 for login, email delivery within 5 minutes
**Scale/Scope**: Initial target 1000 users, designed for 10k+

## Constitution Check

*GATE: All checks passed*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Threat Modeling | ✅ PASS | STRIDE analysis completed → `.osk/specs/002-user-auth/threats.md` |
| II. Risk Analysis | ✅ PASS | 12 risks identified, 3 P0 critical → `.osk/specs/002-user-auth/risks.md` |
| III. Security Requirements | ✅ PASS | 33 FRs mapped to controls in research.md |
| IV. Security Testing | 🔄 PENDING | Tests to be created per task |
| V. Secrets Management | ✅ PASS | JWT_SECRET, TOTP_ENCRYPTION_KEY required (no fallbacks) |
| VI. Audit Logging | ✅ PASS | security_events table designed |
| VII. Patch Management | ✅ PASS | cargo-audit in CI |

### RGPD Compliance

| Article | Requirement | Implementation |
|---------|-------------|----------------|
| Art. 5 | Data minimization | Only essential auth data collected |
| Art. 15-22 | User rights | Account deletion (FR-025), data export planned |
| Art. 25 | Privacy by Design | Password hashed, TOTP encrypted, no email enumeration |
| Art. 32 | Security measures | Argon2id, rate limiting, session management |
| Art. 33-34 | Breach notification | Security events logged for detection |

## Project Structure

### Documentation (this feature)

```text
specs/002-user-auth/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0: Technology decisions
├── data-model.md        # Phase 1: Rust structs and DB schema
├── quickstart.md        # Phase 1: Setup and testing guide
├── contracts/
│   └── auth-api.yaml    # Phase 1: OpenAPI specification
├── checklists/
│   └── requirements.md  # Spec quality validation
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Security Analysis

```text
.osk/specs/002-user-auth/
├── threats.md           # STRIDE threat model (26 threats)
└── risks.md             # Risk register (12 risks, 3 P0)
```

### Source Code (repository root)

```text
src/
├── models/
│   ├── user.rs          # Extend with auth fields
│   ├── auth.rs          # NEW: Login/Register DTOs
│   ├── session.rs       # NEW: Session management
│   ├── password_reset.rs    # NEW: Reset tokens
│   ├── email_confirmation.rs # NEW: Email confirmation
│   ├── recovery_code.rs     # NEW: 2FA recovery
│   ├── security_event.rs    # NEW: Audit logging
│   └── two_factor.rs        # NEW: TOTP DTOs
│
├── services/
│   ├── auth.rs          # NEW: Authentication logic
│   ├── password.rs      # NEW: Hashing, validation, HIBP
│   ├── session.rs       # NEW: Session CRUD
│   ├── email.rs         # NEW: Email sending
│   ├── totp.rs          # NEW: 2FA logic
│   └── security_log.rs  # NEW: Audit events
│
├── api/
│   ├── auth.rs          # NEW: Public auth endpoints
│   ├── account.rs       # NEW: Protected account endpoints
│   └── middleware/
│       └── auth.rs      # UPDATE: Real session validation
│
└── config.rs            # NEW: Validated configuration

migrations/
├── 20251226_001_auth_users.sql     # User table extensions
├── 20251226_002_sessions.sql       # Sessions table
├── 20251226_003_tokens.sql         # Reset/confirmation tokens
├── 20251226_004_recovery_codes.sql # 2FA recovery
└── 20251226_005_security_events.sql # Audit log

tests/
├── integration/
│   ├── auth_test.rs     # Registration, login, logout
│   ├── password_test.rs # Reset, change, validation
│   ├── session_test.rs  # Session management
│   └── totp_test.rs     # 2FA flows
└── security/
    ├── rate_limit_test.rs   # Rate limiting verification
    ├── enumeration_test.rs  # No email/user enumeration
    └── timing_test.rs       # Timing attack resistance
```

**Structure Decision**: Single backend project extending existing Axum API. Auth module is self-contained with clear boundaries.

## Implementation Phases

### Phase 0: Foundation (P0 Critical Risks)

**Focus**: Address RISK-AUTH-001, AUTH-002, AUTH-003

1. **Configuration validation** - No secret fallbacks
2. **Password hashing service** - Argon2id with OWASP params
3. **Session infrastructure** - Secure cookies, DB storage
4. **Rate limiting setup** - Per-IP and per-account

### Phase 1: Core Authentication (P1)

**Focus**: FR-001 to FR-014 (Registration, Login)

1. **Database migrations** - All auth tables
2. **User registration** - Validation, hashing, email confirmation
3. **Login flow** - Credential verification, session creation
4. **Logout** - Session termination

### Phase 2: Password Recovery (P2)

**Focus**: FR-015 to FR-018

1. **Forgot password** - Token generation, email
2. **Reset password** - Token validation, password update

### Phase 3: Session Management (P1)

**Focus**: FR-019 to FR-022

1. **List sessions** - Active session display
2. **Revoke session** - Individual termination
3. **Session metadata** - Device, IP, last activity

### Phase 4: Account Security (P2)

**Focus**: FR-023 to FR-027

1. **Change password** - With session invalidation
2. **Change email** - With confirmation
3. **Security event logging** - All events

### Phase 5: Two-Factor Authentication (P3)

**Focus**: FR-028 to FR-033

1. **TOTP setup** - QR code, secret encryption
2. **TOTP verification** - Login flow integration
3. **Recovery codes** - Generation, single-use

### Phase 6: Account Deletion (P3)

**Focus**: FR-025, FR-026

1. **Request deletion** - 7-day grace period
2. **Cancel deletion** - Reactivation
3. **Execute deletion** - Data removal, recipe handling

## Risk Mitigation Mapping

| Risk ID | Phase | Control Implementation |
|---------|-------|----------------------|
| AUTH-001 | 0 | tower_governor rate limiting |
| AUTH-002 | 0 | argon2 with OWASP params |
| AUTH-003 | 0 | HttpOnly, Secure, SameSite cookies |
| AUTH-004 | 1 | Progressive lockout |
| AUTH-005 | 4 | security_events table |
| AUTH-006 | 2 | 256-bit tokens, 1h expiry |
| AUTH-007 | 5 | AES-256-GCM encrypted secrets |
| AUTH-008 | 1 | Generic errors, constant-time |
| AUTH-009 | 3 | Full session CRUD |
| AUTH-010 | 1 | Fake hash on unknown user |
| AUTH-011 | 1 | Session regeneration on login |
| AUTH-012 | 5 | Bcrypt hashed recovery codes |

## Dependencies to Add

```toml
# Cargo.toml additions
argon2 = "0.5"
totp-rs = { version = "5", features = ["qr", "otpauth", "serde_support"] }
lettre = { version = "0.11", features = ["tokio1-native-tls"] }
aes-gcm = "0.10"
sha1 = "0.10"
hex = "0.4"
lazy_static = "1"
regex = "1"
```

## Success Criteria Mapping

| Criteria | Verification |
|----------|-------------|
| SC-001: Registration < 3 min | E2E test with timer |
| SC-002: Login < 10 sec | Load test p95 |
| SC-003: Password reset < 5 min | E2E test |
| SC-004: 95% registration success | Metrics logging |
| SC-005: Zero recoverable passwords | Code review, security audit |
| SC-006: Brute force blocked | Integration test |
| SC-007: All events logged | Audit table verification |
| SC-008: Session revocation < 1 min | Integration test |
| SC-009: Deletion complete < 24h | Job verification |
| SC-010: 1000 concurrent requests | Load test |

## Next Steps

1. Run `/speckit.tasks` to generate actionable task list
2. Implement Phase 0 first (P0 critical risks)
3. Run `/osk-implement 002-user-auth` after task generation
