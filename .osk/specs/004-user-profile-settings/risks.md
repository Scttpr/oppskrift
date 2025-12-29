# Risk Register: User Profile & Settings Management

**Feature**: 004-user-profile-settings
**Date**: 2025-12-29
**Methodology**: Impact × Probability × Exposure scoring

## Scoring Methodology

**Formula**: `Score = Impact × Probability × Exposure` (1-5 each, max 125)

| Factor | 1 | 2 | 3 | 4 | 5 |
|--------|---|---|---|---|---|
| **Impact** | Negligible | Minor | Moderate | Major | Critical |
| **Probability** | Rare | Unlikely | Possible | Likely | Almost Certain |
| **Exposure** | Internal only | Few users | Some users | Many users | All users |

| Score Range | Severity | Priority | Response |
|-------------|----------|----------|----------|
| ≥ 80 | CRITIQUE | P0 | Block release |
| 49-79 | IMPORTANT | P1 | Fix before release |
| 25-48 | MODÉRÉ | P2 | Fix in next sprint |
| 11-24 | MINEUR | P3 | Backlog |
| 1-10 | FAIBLE | P4 | Accept |

## Risk Summary

| Priority | Count | Status |
|----------|-------|--------|
| P0 (Critical) | 2 | ⏳ Pending |
| P1 (Important) | 3 | ⏳ Pending |
| P2 (Moderate) | 4 | ⏳ Pending |
| P3 (Minor) | 3 | ⏳ Pending |
| P4 (Low) | 2 | Accept |

## Critical Risks (P0)

### RISK-004-001: Account Takeover via Password Change

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-S01, THREAT-004-S03 |
| **Impact** | 5 (Critical - full account compromise) |
| **Probability** | 4 (Likely - credential stuffing common) |
| **Exposure** | 5 (All users with passwords) |
| **Score** | **100** |
| **Severity** | CRITIQUE |
| **Priority** | P0 |

**Description**: Attacker obtains session via credential stuffing or session theft, then changes password to lock out legitimate user.

**Current Controls**:
- Password required for password change (FR-014)
- Session invalidation on password change (FR-017)

**Required Mitigations**:
1. Rate limiting on /api/account/change-password (max 5/hour)
2. Email notification on password change
3. HIBP integration for password breach checking
4. Re-authentication if session > 15 minutes old

**Status**: ⏳ Pending implementation

---

### RISK-004-002: XSS via Profile Fields

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-T01 |
| **Impact** | 5 (Critical - session theft of other users) |
| **Probability** | 3 (Possible - requires XSS vuln) |
| **Exposure** | 5 (All users viewing profiles) |
| **Score** | **75** |
| **Severity** | IMPORTANT |
| **Priority** | P0 |

**Description**: Attacker injects XSS payload in bio or display_name field, stealing sessions of users who view the profile.

**Current Controls**:
- Askama auto-escapes by default

**Required Mitigations**:
1. Verify Askama escaping covers all output contexts
2. Implement Content-Security-Policy header
3. Input validation rejecting HTML/script tags
4. Consider markdown-only bio with sanitization

**Status**: ⏳ Pending implementation

## Important Risks (P1)

### RISK-004-003: 2FA Bypass via Recovery Code Brute Force

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-S04, Attack Tree 2 |
| **Impact** | 4 (Major - bypass 2FA protection) |
| **Probability** | 2 (Unlikely - rate limiting helps) |
| **Exposure** | 4 (Users with 2FA enabled) |
| **Score** | **32** |
| **Severity** | MODÉRÉ |
| **Priority** | P1 |

**Description**: Attacker brute forces recovery codes to bypass 2FA. With 8-character alphanumeric codes, entropy is limited.

**Current Controls**:
- 10 single-use recovery codes

**Required Mitigations**:
1. Rate limit recovery code attempts (5 attempts, then lockout)
2. Long recovery codes (12+ chars with mixed case)
3. Email notification on recovery code use
4. Account lockout after N failed attempts

**Status**: ⏳ Pending implementation

---

### RISK-004-004: TOTP Secret Exposure

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-I03 |
| **Impact** | 5 (Critical - permanent 2FA bypass) |
| **Probability** | 2 (Unlikely - requires code error) |
| **Exposure** | 4 (Users with 2FA enabled) |
| **Score** | **40** |
| **Severity** | MODÉRÉ |
| **Priority** | P1 |

**Description**: TOTP secret accidentally logged or returned in API response, allowing permanent 2FA bypass.

**Current Controls**:
- totp_secret_encrypted stored (encryption at rest)
- Field prefixed with `_` to prevent accidental serialization

**Required Mitigations**:
1. Add #[serde(skip)] to totp_secret field
2. Audit all log statements for sensitive data
3. Add secret scanning to CI/CD
4. Never return decrypted secret except during setup

**Status**: ⏳ Pending implementation

---

### RISK-004-005: Missing Audit Trail for Security Changes

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-R01, THREAT-004-R02 |
| **Impact** | 3 (Moderate - cannot detect compromise) |
| **Probability** | 5 (Almost Certain - no logging exists) |
| **Exposure** | 5 (All users) |
| **Score** | **75** |
| **Severity** | IMPORTANT |
| **Priority** | P1 |

**Description**: Security-sensitive operations (password change, email change, 2FA enable/disable, session revocation) are not logged, preventing incident response.

**Current Controls**: None

**Required Mitigations**:
1. Create security_events table with:
   - event_type, user_id, ip_address, user_agent, timestamp
2. Log all security-sensitive operations
3. Email notification for high-risk changes
4. Admin dashboard for security events

**Status**: ⏳ Pending implementation

## Moderate Risks (P2)

### RISK-004-006: Session Hijacking via Insecure Cookie

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-S01 |
| **Impact** | 5 (Critical) |
| **Probability** | 2 (Unlikely - HTTPS enforced) |
| **Exposure** | 5 (All users) |
| **Score** | **50** |
| **Severity** | IMPORTANT |
| **Priority** | P2 |

**Description**: Session cookie stolen if not properly secured.

**Current Controls**:
- HttpOnly flag (prevents JS access)

**Required Mitigations**:
1. Verify Secure flag (HTTPS only)
2. Add SameSite=Lax or Strict
3. Consider session binding to IP/fingerprint

**Status**: ⏳ Pending verification

---

### RISK-004-007: Email Enumeration

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-I01 |
| **Impact** | 2 (Minor - privacy concern) |
| **Probability** | 4 (Likely - easy to test) |
| **Exposure** | 5 (All users) |
| **Score** | **40** |
| **Severity** | MODÉRÉ |
| **Priority** | P2 |

**Description**: Attacker can determine if email exists via response timing or message differences on email change.

**Current Controls**: None

**Required Mitigations**:
1. Generic response: "If this email exists, verification sent"
2. Consistent response times (add delay if needed)

**Status**: ⏳ Pending implementation

---

### RISK-004-008: CSRF on Settings Forms

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-T03 |
| **Impact** | 4 (Major - unauthorized changes) |
| **Probability** | 2 (Unlikely - SameSite helps) |
| **Exposure** | 5 (All users) |
| **Score** | **40** |
| **Severity** | MODÉRÉ |
| **Priority** | P2 |

**Description**: Attacker tricks user into submitting settings form via malicious page.

**Current Controls**:
- SameSite cookie (assumed)

**Required Mitigations**:
1. CSRF tokens on all forms
2. Re-verify SameSite cookie setting
3. Require password for destructive actions

**Status**: ⏳ Pending verification

---

### RISK-004-009: Session Exhaustion DoS

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-D02 |
| **Impact** | 2 (Minor - degraded performance) |
| **Probability** | 3 (Possible) |
| **Exposure** | 5 (All users) |
| **Score** | **30** |
| **Severity** | MODÉRÉ |
| **Priority** | P2 |

**Description**: Attacker creates many sessions per user, exhausting database resources.

**Current Controls**: None

**Required Mitigations**:
1. Limit sessions per user (10 max)
2. Oldest session invalidation on limit
3. Rate limiting on login

**Status**: ⏳ Pending implementation

## Minor Risks (P3)

### RISK-004-010: IDOR on Session Revocation

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-E01 |
| **Impact** | 3 (Moderate) |
| **Probability** | 1 (Rare - UUIDs used) |
| **Exposure** | 5 (All users) |
| **Score** | **15** |
| **Severity** | MINEUR |
| **Priority** | P3 |

**Description**: User could revoke another user's session if authorization check missing.

**Current Controls**:
- UUID session IDs (unpredictable)

**Required Mitigations**:
1. Verify session.user_id == auth.user_id before revocation
2. Add integration test for this check

**Status**: ⏳ Pending verification

---

### RISK-004-011: Password Hash Exposure

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-I04 |
| **Impact** | 5 (Critical) |
| **Probability** | 1 (Rare - basic error) |
| **Exposure** | 5 (All users) |
| **Score** | **25** |
| **Severity** | MODÉRÉ |
| **Priority** | P3 |

**Description**: Password hash accidentally returned in API response.

**Current Controls**:
- UserProfile DTO excludes password_hash
- #[serde(skip)] likely present

**Required Mitigations**:
1. Verify #[serde(skip)] on password_hash
2. Add test asserting password_hash not in response

**Status**: ⏳ Pending verification

---

### RISK-004-012: Admin Impersonation via Display Name

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-E03 |
| **Impact** | 2 (Minor - social engineering) |
| **Probability** | 3 (Possible) |
| **Exposure** | 3 (Users viewing profiles) |
| **Score** | **18** |
| **Severity** | MINEUR |
| **Priority** | P3 |

**Description**: User sets display_name to "Admin" or similar to mislead others.

**Current Controls**: None

**Required Mitigations**:
1. Block reserved words in display_name (admin, moderator, staff)
2. Show verified badge for actual admins

**Status**: ⏳ Pending evaluation

## Low Risks (P4 - Accept)

### RISK-004-013: Email Change Token Interception

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-S02 |
| **Impact** | 4 (Major) |
| **Probability** | 1 (Rare - requires email compromise) |
| **Exposure** | 3 (Users changing email) |
| **Score** | **12** |
| **Severity** | MINEUR |
| **Priority** | P4 |

**Status**: ✅ Accept - Email security is user responsibility

---

### RISK-004-014: Session Info Leakage

| Attribute | Value |
|-----------|-------|
| **Threat** | THREAT-004-I02 |
| **Impact** | 1 (Negligible - metadata only) |
| **Probability** | 2 (Unlikely) |
| **Exposure** | 3 (Users viewing sessions) |
| **Score** | **6** |
| **Severity** | FAIBLE |
| **Priority** | P4 |

**Status**: ✅ Accept - IP/User-Agent needed for session identification

## Conformité

### RGPD (Applicable)

| Requirement | Status | Notes |
|-------------|--------|-------|
| Art. 17 - Right to Erasure | ✅ | Account deletion with content choice |
| Art. 32 - Security | ⏳ | Pending mitigations above |
| Art. 33 - Breach Notification | ⏳ | Requires audit logging (RISK-004-005) |

### RGS (Not applicable)

This feature does not handle government data.

## Summary Statistics

- **Total Risks**: 14
- **Critical (P0)**: 2
- **Important (P1)**: 3
- **Moderate (P2)**: 4
- **Minor (P3)**: 3
- **Low (P4)**: 2 (accepted)

**Risk Score Total**: 578
**Average Score**: 41.3 (Moderate)

## Next Steps

1. Address P0 risks before feature development
2. Include P1 mitigations in implementation tasks
3. Review P2 during code review
4. Document accepted risks (P4) in security decision log
