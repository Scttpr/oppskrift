# Threat Model: User Profile & Settings Management

**Feature**: 004-user-profile-settings
**Date**: 2025-12-29
**Methodology**: STRIDE + Attack Trees

## Executive Summary

This feature extends the attack surface of user account management by exposing settings interfaces for profile editing, password changes, email changes, 2FA management, session management, and account deletion. Critical assets include authentication credentials, session tokens, and personal data.

## Assets

| Asset | Sensitivity | Impact if Compromised |
|-------|-------------|----------------------|
| User password hash | Critical | Full account takeover |
| Session tokens | Critical | Session hijacking |
| TOTP secret | Critical | 2FA bypass |
| Recovery codes | Critical | 2FA bypass |
| Email address | High | Account recovery attacks |
| Profile data (bio, display_name) | Medium | Privacy violation, impersonation |
| User preferences | Low | Minor UX impact |

## Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        TRUST BOUNDARY: Internet                  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    [Browser/Client]                              │
│  - Session cookie (oppskrift_session)                           │
│  - CSRF token                                                    │
└─────────────────────────────────────────────────────────────────┘
                                │ HTTPS
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                 TRUST BOUNDARY: Application                      │
├─────────────────────────────────────────────────────────────────┤
│  [Axum Web Server]                                              │
│  ├── /settings/* handlers (HTML)                                │
│  ├── /api/account/* endpoints (JSON)                            │
│  └── AuthUser middleware (session validation)                   │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   [Service Layer]                                │
│  ├── UserService (profile CRUD)                                 │
│  ├── SessionService (session management)                        │
│  └── TotpService (2FA operations)                               │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                 TRUST BOUNDARY: Database                         │
├─────────────────────────────────────────────────────────────────┤
│  [PostgreSQL]                                                    │
│  ├── users (password_hash, totp_secret_encrypted)               │
│  └── sessions (token_hash)                                      │
└─────────────────────────────────────────────────────────────────┘
```

## STRIDE Analysis

### S - Spoofing

#### THREAT-004-S01: Session Token Theft
**Description**: Attacker steals session cookie via XSS, network sniffing, or malware
**Attack Vector**: XSS injection, HTTP (non-HTTPS), browser extension malware
**Assets Affected**: Session tokens
**Likelihood**: Medium (requires XSS or network access)
**Impact**: Critical (full account access)

#### THREAT-004-S02: Password Reset Token Interception
**Description**: Attacker intercepts email change verification email
**Attack Vector**: Email account compromise, MitM on email
**Assets Affected**: Email change tokens
**Likelihood**: Low (requires email compromise)
**Impact**: High (can change email, then reset password)

#### THREAT-004-S03: Credential Stuffing on Password Change
**Description**: Attacker uses leaked credentials to change password
**Attack Vector**: Credential database leaks
**Assets Affected**: User accounts
**Likelihood**: Medium (credential leaks common)
**Impact**: Critical (account takeover)

#### THREAT-004-S04: TOTP Replay Attack
**Description**: Attacker captures and replays valid TOTP code within window
**Attack Vector**: Shoulder surfing, screen recording
**Assets Affected**: 2FA protection
**Likelihood**: Low (narrow time window)
**Impact**: High (2FA bypass)

### T - Tampering

#### THREAT-004-T01: Profile Data Injection
**Description**: Attacker injects malicious content in bio/display_name
**Attack Vector**: XSS payload in profile fields
**Assets Affected**: Profile data, other users viewing profile
**Likelihood**: Medium (common attack)
**Impact**: Medium (XSS can steal other users' sessions)

#### THREAT-004-T02: Email Change to Attacker-Controlled Address
**Description**: Attacker changes victim's email after gaining session access
**Attack Vector**: Session hijacking → email change
**Assets Affected**: Account recovery mechanism
**Likelihood**: Low (requires session compromise first)
**Impact**: Critical (can lock out legitimate user)

#### THREAT-004-T03: Deletion Content Choice Manipulation
**Description**: Attacker modifies deletion request to delete vs anonymize
**Attack Vector**: Parameter tampering, CSRF
**Assets Affected**: User content
**Likelihood**: Low (requires authentication)
**Impact**: High (irreversible content loss)

### R - Repudiation

#### THREAT-004-R01: Untracked Security Changes
**Description**: Password/email/2FA changes without audit trail
**Attack Vector**: Administrative or attacker actions
**Assets Affected**: Security posture, forensics
**Likelihood**: N/A (design gap)
**Impact**: Medium (cannot detect compromise)

#### THREAT-004-R02: Session Activity Without Logging
**Description**: Session creation/revocation not logged
**Attack Vector**: N/A
**Assets Affected**: Forensic capability
**Likelihood**: N/A (design gap)
**Impact**: Medium (cannot trace unauthorized access)

### I - Information Disclosure

#### THREAT-004-I01: Email Address Enumeration
**Description**: Attacker determines if email exists via email change response
**Attack Vector**: Timing attacks, error message differences
**Assets Affected**: User email addresses
**Likelihood**: Medium (common in web apps)
**Impact**: Low (privacy concern, phishing target list)

#### THREAT-004-I02: Session List Information Leakage
**Description**: Detailed session info (IP, user agent) visible to attacker with session access
**Attack Vector**: Session hijacking
**Assets Affected**: User location/device information
**Likelihood**: Low (requires session access)
**Impact**: Low (metadata only)

#### THREAT-004-I03: TOTP Secret Exposure
**Description**: TOTP secret leaked via logs, error messages, or API response
**Attack Vector**: Verbose logging, improper error handling
**Assets Affected**: 2FA protection
**Likelihood**: Low (coding error)
**Impact**: Critical (permanent 2FA bypass)

#### THREAT-004-I04: Password Hash Exposure
**Description**: Password hash returned in API response or logged
**Attack Vector**: Improper serialization, logging
**Assets Affected**: Password security
**Likelihood**: Low (coding error)
**Impact**: Critical (offline cracking possible)

### D - Denial of Service

#### THREAT-004-D01: Account Lockout via Deletion Request
**Description**: Attacker requests deletion, user loses access during grace period
**Attack Vector**: Session hijacking → deletion request
**Assets Affected**: Account availability
**Likelihood**: Low (requires session access)
**Impact**: Medium (temporary, can cancel)

#### THREAT-004-D02: Session Exhaustion
**Description**: Attacker creates many sessions to exhaust resources
**Attack Vector**: Automated login requests
**Assets Affected**: Server resources, session table
**Likelihood**: Medium (easy to automate)
**Impact**: Low (rate limiting mitigates)

### E - Elevation of Privilege

#### THREAT-004-E01: IDOR on Session Revocation
**Description**: User revokes another user's session via predictable ID
**Attack Vector**: Sequential/predictable session IDs
**Assets Affected**: Other users' sessions
**Likelihood**: Low (UUIDs used)
**Impact**: High (force other users to re-login)

#### THREAT-004-E02: Profile Update Without Authentication Check
**Description**: Missing auth check allows unauthenticated profile modification
**Attack Vector**: Direct API call without session
**Assets Affected**: All user profiles
**Likelihood**: Very Low (basic auth middleware exists)
**Impact**: Critical (mass account compromise)

#### THREAT-004-E03: Admin Impersonation via Profile Manipulation
**Description**: User sets display_name to mimic admin
**Attack Vector**: Social engineering via profile fields
**Assets Affected**: User trust
**Likelihood**: Medium (no technical barrier)
**Impact**: Low (visual only, no privilege escalation)

## Attack Trees

### Attack Tree 1: Account Takeover via Password Change

```
[GOAL: Change victim's password]
├── 1. Obtain valid session
│   ├── 1.1 Steal session cookie
│   │   ├── 1.1.1 XSS attack (requires vuln)
│   │   ├── 1.1.2 Network sniffing (requires HTTP)
│   │   └── 1.1.3 Malware on victim device
│   ├── 1.2 Session fixation (requires vuln)
│   └── 1.3 Credential stuffing (requires leaked creds)
│
├── 2. Know current password OR bypass
│   ├── 2.1 Credential stuffing (same leaked creds)
│   ├── 2.2 Social engineering
│   └── 2.3 Exploit password change without current password (vuln)
│
└── 3. Execute password change
    ├── 3.1 POST /api/account/change-password
    └── 3.2 All other sessions invalidated (FR-017)
```

### Attack Tree 2: 2FA Bypass

```
[GOAL: Access account with 2FA enabled]
├── 1. Disable 2FA
│   ├── 1.1 Obtain session + password
│   │   └── 1.1.1 Credential stuffing
│   ├── 1.2 Brute force current TOTP code
│   │   └── Limited by rate limiting
│   └── 1.3 Use recovery code
│       └── 1.3.1 Brute force recovery codes
│
├── 2. Bypass 2FA
│   ├── 2.1 TOTP secret theft
│   │   ├── 2.1.1 Database compromise
│   │   └── 2.1.2 Log file exposure
│   └── 2.2 Timing attack on TOTP validation
│
└── 3. Account recovery bypass
    └── 3.1 Email change + password reset (requires session)
```

## Mitigations Required

| Threat ID | Mitigation | Priority |
|-----------|------------|----------|
| THREAT-004-S01 | HttpOnly, Secure, SameSite cookies; CSP headers | P0 |
| THREAT-004-S03 | Rate limiting on password change; breach detection | P1 |
| THREAT-004-T01 | Input sanitization; output encoding; CSP | P0 |
| THREAT-004-T02 | Require password for email change (already in spec) | P0 |
| THREAT-004-T03 | CSRF protection; re-authenticate for deletion | P1 |
| THREAT-004-R01 | Audit logging for security-sensitive operations | P1 |
| THREAT-004-I01 | Consistent response times; generic error messages | P2 |
| THREAT-004-I03 | Never log/return TOTP secrets; sanitize error responses | P0 |
| THREAT-004-I04 | Use #[serde(skip)] on password_hash; sanitize logs | P0 |
| THREAT-004-D02 | Session limit per user (e.g., 10 max) | P2 |
| THREAT-004-E01 | Validate session belongs to authenticated user | P0 |
| THREAT-004-E02 | Ensure AuthUser middleware on all routes | P0 |

## References

- OWASP Authentication Cheat Sheet
- OWASP Session Management Cheat Sheet
- NIST SP 800-63B Digital Identity Guidelines
