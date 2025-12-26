# Tasks: User Authentication

**Input**: Design documents from `/specs/002-user-auth/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/auth-api.yaml

**Tests**: Security tests included due to the security-critical nature of authentication. Standard unit/integration tests follow TDD pattern.

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1-US5)
- Include exact file paths in descriptions

## Path Conventions

Single project structure at repository root:
- Source: `src/`
- Tests: `tests/`
- Migrations: `migrations/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, dependencies, and configuration

- [x] T001 Add authentication dependencies to Cargo.toml (argon2, totp-rs, lettre, aes-gcm, sha1, hex, lazy_static, regex)
- [x] T002 [P] Create src/config.rs with validated configuration (JWT_SECRET, TOTP_ENCRYPTION_KEY required, no fallbacks)
- [ ] T003 [P] Update src/lib.rs to export new auth modules (config, models, services, api)
- [x] T004 [P] Create .env.example with all required auth environment variables

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Database Migrations

- [ ] T005 Create migrations/20251226_001_auth_users.sql - Add auth fields to users table (email, email_verified, password_hash, totp fields, lockout fields, deletion_requested_at)
- [ ] T006 [P] Create migrations/20251226_002_sessions.sql - Sessions table with token_hash, device_info, ip_address, user_agent, timestamps
- [ ] T007 [P] Create migrations/20251226_003_tokens.sql - password_reset_tokens and email_confirmation_tokens tables
- [ ] T008 [P] Create migrations/20251226_004_recovery_codes.sql - recovery_codes table for 2FA
- [ ] T009 [P] Create migrations/20251226_005_security_events.sql - Audit log table with indexes

### Core Services (Foundation)

- [ ] T010 Create src/services/password.rs - Argon2id hashing with OWASP params, password strength validation, HIBP breach check
- [ ] T011 [P] Create src/services/security_log.rs - Security event logging service with CreateSecurityEvent and all SecurityEventType variants
- [ ] T012 [P] Create src/services/email.rs - Email service with lettre for sending confirmation, reset, and notification emails
- [ ] T013 Create src/services/session.rs - Session CRUD: create, validate, revoke, cleanup, with secure cookie handling

### Auth Models (Shared)

- [ ] T014 Update src/models/user.rs - Extend User struct with auth fields per data-model.md, add RESERVED_USERNAMES const
- [ ] T015 [P] Create src/models/auth.rs - RegisterRequest, RegisterResponse, LoginRequest, LoginResponse, TwoFactorRequired DTOs with validation
- [ ] T016 [P] Create src/models/session.rs - Session, SessionInfo, CreateSession structs per data-model.md
- [ ] T017 [P] Create src/models/security_event.rs - SecurityEventType enum, SecurityEvent, CreateSecurityEvent structs

### Middleware

- [ ] T018 Update src/api/middleware/auth.rs - Replace stub with real session validation, extract AuthUser from session cookie

### Rate Limiting

- [ ] T019 Configure tower_governor for auth endpoints in src/main.rs - Per-IP (10/min) and per-account (5 attempts) rate limiting

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - New User Registration (Priority: P1) 🎯 MVP

**Goal**: Enable visitors to create accounts with email/password, receive confirmation email, and activate account

**Independent Test**: Register new account → receive confirmation email → click link → account activated

### Models for User Story 1

- [ ] T020 [P] [US1] Create src/models/email_confirmation.rs - EmailConfirmationToken struct per data-model.md

### Services for User Story 1

- [ ] T021 [US1] Create src/services/auth.rs - register() function: validate input, check username/email uniqueness, hash password, create unverified user, generate confirmation token, send email
- [ ] T022 [US1] Add confirm_email() to src/services/auth.rs - Validate token, mark user email_verified, log security event
- [ ] T023 [US1] Add resend_confirmation() to src/services/auth.rs - Generate new token, invalidate old, send email

### API Endpoints for User Story 1

- [ ] T024 [US1] Create src/api/auth.rs - POST /api/auth/register endpoint with validation, rate limiting
- [ ] T025 [US1] Add GET /api/auth/confirm-email/:token to src/api/auth.rs - Token validation, account activation
- [ ] T026 [US1] Add POST /api/auth/resend-confirmation to src/api/auth.rs - Resend confirmation email

### Integration for User Story 1

- [ ] T027 [US1] Register auth routes in src/main.rs Router - Mount /api/auth/* endpoints with rate limiting layer
- [ ] T028 [US1] Add email templates in src/services/email.rs - confirmation_email_template() function

### Tests for User Story 1

- [ ] T029 [P] [US1] Create tests/integration/registration_test.rs - Test successful registration, duplicate email/username, password validation, confirmation flow
- [ ] T030 [P] [US1] Add security test in tests/security/enumeration_test.rs - Verify registration doesn't reveal existing emails

**Checkpoint**: User Story 1 complete - users can register and confirm accounts

---

## Phase 4: User Story 2 - Returning User Login (Priority: P1)

**Goal**: Enable registered users to log in with email/password, receive session cookie, and access protected content

**Independent Test**: Login with valid credentials → receive session → access protected endpoint → logout

### Services for User Story 2

- [ ] T031 [US2] Add login() to src/services/auth.rs - Verify credentials, check lockout, create session, log event, handle 2FA check (defer actual 2FA to US4)
- [ ] T032 [US2] Add logout() to src/services/auth.rs - Terminate current session, log event
- [ ] T033 [US2] Add check_lockout() and increment_failed_attempts() to src/services/auth.rs - Account lockout logic (5 attempts, 15-min lockout)

### API Endpoints for User Story 2

- [ ] T034 [US2] Add POST /api/auth/login to src/api/auth.rs - Credential verification, session creation, secure cookie
- [ ] T035 [US2] Add POST /api/auth/logout to src/api/auth.rs - Session termination
- [ ] T036 [US2] Create src/api/account.rs - GET /api/account/profile endpoint (protected, uses AuthUser)

### Integration for User Story 2

- [ ] T037 [US2] Update src/main.rs - Add account routes with session auth requirement
- [ ] T038 [US2] Implement constant-time credential verification in src/services/auth.rs - Fake hash check for non-existent users to prevent timing attacks

### Tests for User Story 2

- [ ] T039 [P] [US2] Create tests/integration/login_test.rs - Test successful login, invalid credentials, lockout after 5 failures, logout
- [ ] T040 [P] [US2] Create tests/security/rate_limit_test.rs - Verify rate limiting blocks brute force
- [ ] T041 [P] [US2] Add timing test in tests/security/timing_test.rs - Verify constant-time response for valid vs invalid users

**Checkpoint**: User Stories 1 & 2 complete - functional registration and login (MVP!)

---

## Phase 5: User Story 3 - Password Recovery (Priority: P2)

**Goal**: Enable users to reset forgotten passwords via email link

**Independent Test**: Request reset → receive email → click link → set new password → login with new password

### Models for User Story 3

- [ ] T042 [P] [US3] Create src/models/password_reset.rs - PasswordResetToken, ForgotPasswordRequest, ResetPasswordRequest structs per data-model.md

### Services for User Story 3

- [ ] T043 [US3] Add forgot_password() to src/services/auth.rs - Generate token, send email (same response regardless of email existence)
- [ ] T044 [US3] Add reset_password() to src/services/auth.rs - Validate token (1h expiry, single-use), hash new password, invalidate all sessions, log event

### API Endpoints for User Story 3

- [ ] T045 [US3] Add POST /api/auth/forgot-password to src/api/auth.rs - Request password reset
- [ ] T046 [US3] Add POST /api/auth/reset-password to src/api/auth.rs - Set new password with token

### Integration for User Story 3

- [ ] T047 [US3] Add password_reset_email_template() to src/services/email.rs

### Tests for User Story 3

- [ ] T048 [P] [US3] Create tests/integration/password_reset_test.rs - Test full reset flow, expired token, used token, invalid token
- [ ] T049 [P] [US3] Add enumeration test - Verify same response for registered/unregistered emails

**Checkpoint**: User Story 3 complete - users can recover forgotten passwords

---

## Phase 6: User Story 4 - Account Security Settings (Priority: P3)

**Goal**: Enable users to manage security: change password, view/revoke sessions, enable/disable 2FA

**Independent Test**: Change password → other sessions terminated | Enable 2FA → login requires TOTP

### Models for User Story 4

- [ ] T050 [P] [US4] Create src/models/two_factor.rs - TwoFactorSetupResponse, EnableTwoFactorRequest, DisableTwoFactorRequest structs
- [ ] T051 [P] [US4] Create src/models/recovery_code.rs - RecoveryCode, RecoveryCodesResponse, RecoveryCodesStatus structs
- [ ] T052 [P] [US4] Create src/models/account.rs - ChangePasswordRequest, ChangeEmailRequest structs

### Services for User Story 4

- [ ] T053 [US4] Add change_password() to src/services/auth.rs - Verify current password, hash new, invalidate other sessions, log event
- [ ] T054 [US4] Add change_email() to src/services/auth.rs - Generate confirmation token for new email, keep old until confirmed
- [ ] T055 [US4] Add list_sessions() and revoke_session() to src/services/session.rs - Session management
- [ ] T056 [US4] Create src/services/totp.rs - setup_2fa(), enable_2fa(), disable_2fa(), verify_totp(), encrypt/decrypt secret with AES-256-GCM
- [ ] T057 [US4] Add generate_recovery_codes() and use_recovery_code() to src/services/totp.rs - 8 bcrypt-hashed codes, single-use

### API Endpoints for User Story 4

- [ ] T058 [US4] Add POST /api/account/change-password to src/api/account.rs
- [ ] T059 [US4] Add POST /api/account/change-email to src/api/account.rs
- [ ] T060 [US4] Add GET /api/account/sessions to src/api/account.rs
- [ ] T061 [US4] Add DELETE /api/account/sessions/:id to src/api/account.rs
- [ ] T062 [US4] Add POST /api/account/2fa/setup to src/api/account.rs - Returns QR code and secret
- [ ] T063 [US4] Add POST /api/account/2fa/enable to src/api/account.rs - Verify TOTP, return recovery codes
- [ ] T064 [US4] Add POST /api/account/2fa/disable to src/api/account.rs - Require password + TOTP
- [ ] T065 [US4] Add GET /api/account/2fa/recovery-codes to src/api/account.rs - Remaining count
- [ ] T066 [US4] Add POST /api/account/2fa/recovery-codes to src/api/account.rs - Regenerate codes

### Integration for User Story 4

- [ ] T067 [US4] Update login() in src/services/auth.rs - Integrate TOTP verification for users with 2FA enabled
- [ ] T068 [US4] Add security notification emails - password_changed, 2fa_enabled, 2fa_disabled templates

### Tests for User Story 4

- [ ] T069 [P] [US4] Create tests/integration/session_test.rs - Test list sessions, revoke session, session invalidation on password change
- [ ] T070 [P] [US4] Create tests/integration/totp_test.rs - Test 2FA setup, enable, verify, disable, recovery codes
- [ ] T071 [P] [US4] Add rate limit test for TOTP verification - Max 3 attempts per 10 minutes

**Checkpoint**: User Story 4 complete - full account security management including 2FA

---

## Phase 7: User Story 5 - Account Deletion (Priority: P3)

**Goal**: Enable users to delete their account with 7-day grace period (GDPR compliance)

**Independent Test**: Request deletion → grace period starts → cancel OR wait 7 days → account deleted

### Models for User Story 5

- [ ] T072 [P] [US5] Add DeleteAccountRequest, DeletionScheduledResponse to src/models/account.rs

### Services for User Story 5

- [ ] T073 [US5] Add request_deletion() to src/services/auth.rs - Set deletion_requested_at, log event
- [ ] T074 [US5] Add cancel_deletion() to src/services/auth.rs - Clear deletion_requested_at, log event
- [ ] T075 [US5] Add execute_deletion() to src/services/auth.rs - Hard delete user data, anonymize/delete recipes per user choice

### API Endpoints for User Story 5

- [ ] T076 [US5] Add POST /api/account/delete to src/api/account.rs - Request deletion with password confirmation
- [ ] T077 [US5] Add POST /api/account/cancel-deletion to src/api/account.rs - Cancel during grace period

### Background Jobs for User Story 5

- [ ] T078 [US5] Create src/jobs/cleanup.rs - Scheduled job to execute deletions after grace period, cleanup expired tokens/sessions

### Tests for User Story 5

- [ ] T079 [P] [US5] Create tests/integration/deletion_test.rs - Test request, cancel, execute after grace period

**Checkpoint**: User Story 5 complete - GDPR-compliant account deletion

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T080 [P] Update specs/002-user-auth/quickstart.md - Add actual curl examples with working endpoints
- [ ] T081 [P] Add OpenAPI documentation to endpoints using utoipa macros in src/api/auth.rs and src/api/account.rs
- [ ] T082 Implement session cleanup job in src/jobs/cleanup.rs - Remove expired sessions daily
- [ ] T083 [P] Add security event export endpoint GET /api/account/security-events (audit trail for users)
- [ ] T084 Review and harden all error messages - Ensure no information leakage
- [ ] T085 Run cargo clippy and cargo fmt - Code quality pass
- [ ] T086 Run cargo audit - Check for security vulnerabilities in dependencies
- [ ] T087 Manual security review - Verify all OWASP controls from .osk/specs/002-user-auth/risks.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - US1 & US2 (P1) are MVP - implement first
  - US3 (P2) can follow or parallel with US1/US2
  - US4 & US5 (P3) can proceed after P1/P2
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (Registration, P1)**: After Foundational - No dependencies on other stories
- **User Story 2 (Login, P1)**: After Foundational - Independent but pairs well with US1
- **User Story 3 (Password Recovery, P2)**: After Foundational - Independent
- **User Story 4 (Security Settings, P3)**: After Foundational - Independent, but 2FA login integration touches US2 code
- **User Story 5 (Account Deletion, P3)**: After Foundational - Independent

### Within Each User Story

- Models before services
- Services before endpoints
- Core implementation before integration
- Tests can run after implementation

### Parallel Opportunities

- All [P] tasks within a phase can run in parallel
- US1 and US2 can be developed in parallel (different files)
- US3, US4, US5 can all proceed in parallel after P1/P2
- Multiple models marked [P] can be created simultaneously

---

## Parallel Example: Phase 2 (Foundational)

```bash
# Launch all migrations together:
Task T006: "Create migrations/20251226_002_sessions.sql"
Task T007: "Create migrations/20251226_003_tokens.sql"
Task T008: "Create migrations/20251226_004_recovery_codes.sql"
Task T009: "Create migrations/20251226_005_security_events.sql"

# Launch parallel services:
Task T011: "Create src/services/security_log.rs"
Task T012: "Create src/services/email.rs"

# Launch parallel models:
Task T015: "Create src/models/auth.rs"
Task T016: "Create src/models/session.rs"
Task T017: "Create src/models/security_event.rs"
```

---

## Parallel Example: User Story 1

```bash
# Models first (parallel):
Task T020: "Create src/models/email_confirmation.rs"

# Then services (sequential - depend on models):
Task T021: "Create src/services/auth.rs - register()"
Task T022: "Add confirm_email() to src/services/auth.rs"
Task T023: "Add resend_confirmation() to src/services/auth.rs"

# Then endpoints (sequential - depend on services):
Task T024: "Create src/api/auth.rs - POST /api/auth/register"
Task T025: "Add GET /api/auth/confirm-email/:token"
Task T026: "Add POST /api/auth/resend-confirmation"

# Tests can run in parallel after implementation:
Task T029: "tests/integration/registration_test.rs"
Task T030: "tests/security/enumeration_test.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1 & 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Registration)
4. Complete Phase 4: User Story 2 (Login)
5. **STOP and VALIDATE**: Test registration + login independently
6. Deploy/demo if ready - users can register and login!

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → **Milestone: Registration works**
3. Add User Story 2 → Test independently → **Milestone: Full auth MVP**
4. Add User Story 3 → Test independently → **Milestone: Password recovery**
5. Add User Story 4 → Test independently → **Milestone: 2FA + session management**
6. Add User Story 5 → Test independently → **Milestone: GDPR compliance**
7. Polish phase → **Milestone: Production ready**

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Registration)
   - Developer B: User Story 2 (Login)
3. After P1 stories complete:
   - Developer A: User Story 3 (Password Recovery)
   - Developer B: User Story 4 (Security Settings)
   - Developer C: User Story 5 (Account Deletion)
4. All stories integrate independently

---

## Summary

| Phase | Tasks | Parallel Opportunities |
|-------|-------|----------------------|
| Setup | 4 | 3 parallel (T002-T004) |
| Foundational | 15 | 11 parallel |
| US1 Registration | 11 | 3 parallel tests |
| US2 Login | 11 | 3 parallel tests |
| US3 Password Recovery | 8 | 2 parallel tests |
| US4 Security Settings | 22 | 6 parallel |
| US5 Account Deletion | 8 | 1 parallel test |
| Polish | 8 | 4 parallel |
| **Total** | **87** | **~33 parallel groups** |

### Task Count by User Story

- **US1 (Registration)**: 11 tasks
- **US2 (Login)**: 11 tasks
- **US3 (Password Recovery)**: 8 tasks
- **US4 (Security Settings)**: 22 tasks
- **US5 (Account Deletion)**: 8 tasks

### Independent Test Criteria

- **US1**: Register → Confirm email → Account active
- **US2**: Login → Session created → Access protected → Logout
- **US3**: Forgot password → Reset email → New password works
- **US4**: Change password → Sessions invalidated | 2FA → Login requires TOTP
- **US5**: Request deletion → Grace period → Cancel OR Delete

### Suggested MVP Scope

**User Stories 1 + 2 only** (22 tasks + 19 foundational = 41 tasks for functional auth)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Security tests are included due to authentication being security-critical
- All risks from .osk/specs/002-user-auth/risks.md are addressed in task implementations
