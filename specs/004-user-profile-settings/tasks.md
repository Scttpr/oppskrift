# Tasks: User Profile & Settings Management

**Input**: Design documents from `/specs/004-user-profile-settings/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/api.yaml, .osk/specs/004-user-profile-settings/

**Tests**: Not explicitly requested - focusing on implementation tasks with security mitigations from OSK analysis.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

**Status**: ✅ COMPLETE (2025-12-29)

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project**: `src/`, `templates/`, `tests/` at repository root
- Tech stack: Rust 1.75+, Axum 0.8, Askama, SQLx, PostgreSQL 15+

---

## Phase 1: Setup ✅

**Purpose**: Create settings module structure and templates directory

- [x] T001 Create settings handler module skeleton in src/handlers/settings.rs
- [x] T002 [P] Create templates/settings/ directory structure
- [x] T003 [P] Create settings navigation partial in templates/settings/_nav.html
- [x] T004 [P] Create settings layout template in templates/settings/_layout.html
- [x] T005 Add settings routes to src/handlers/mod.rs

---

## Phase 2: Foundational (Blocking Prerequisites) ✅

**Purpose**: Security infrastructure and data model extensions required by OSK analysis

**CRITICAL**: No user story work can begin until this phase is complete (addresses P0/P1 risks)

- [x] T006 Create database migration for deletion_content_choice enum in migrations/20251229000001_add_deletion_content_choice.sql
- [x] T007 Add DeletionContentChoice enum to src/models/user.rs
- [x] T008 [P] Verify #[serde(skip)] on password_hash in src/models/user.rs (RISK-004-011)
- [x] T009 [P] Verify #[serde(skip)] on totp_secret_encrypted in src/models/user.rs (RISK-004-004)
- [x] T010 [P] Verify session cookie has Secure and SameSite flags in src/api/middleware/auth.rs (RISK-004-006)
- [x] T011 Add Content-Security-Policy header middleware in src/api/middleware/security.rs (RISK-004-002)
- [x] T012 [P] Add CSRF token generation and validation utilities in src/lib/csrf.rs (RISK-004-008)
- [x] T013 Implement email masking helper function in src/lib/helpers.rs
- [x] T014 Create security_events table migration in migrations/20251226000019_security_events.sql (RISK-004-005)
- [x] T015 Create SecurityEvent model in src/models/security_event.rs
- [x] T016 Implement AuditService for logging security operations in src/services/audit_service.rs
- [x] T017 Run migrations and regenerate SQLx offline data

**Checkpoint**: Foundation ready - user story implementation can now begin ✅

---

## Phase 3: User Story 1 - View My Profile (Priority: P1) ✅

**Goal**: Display authenticated user's complete profile information on a dedicated settings page

**Independent Test**: Log in, navigate to /settings/profile, verify all user fields displayed with masked email

### Implementation for User Story 1

- [x] T018 [US1] Create UserProfile view struct with masked email in src/handlers/settings.rs
- [x] T019 [US1] Create ProfileTemplate struct with Askama derive in src/handlers/settings.rs
- [x] T020 [US1] Implement GET /settings redirect to /settings/profile handler in src/handlers/settings.rs
- [x] T021 [US1] Implement GET /settings/profile handler fetching user data in src/handlers/settings.rs
- [x] T022 [US1] Create profile view template in templates/settings/profile.html
- [x] T023 [US1] Add accessible form labels and ARIA attributes to profile template (WCAG 2.1 AA)
- [x] T024 [US1] Verify AuthUser middleware applied to all settings routes (RISK-004-E02)

**Checkpoint**: User Story 1 complete - users can view their profile ✅

---

## Phase 4: User Story 2 - Edit Profile Information (Priority: P1) ✅

**Goal**: Allow users to edit display_name, bio, avatar_url, measurement_pref

**Independent Test**: Edit each field, save, verify changes persist across page reload

### Implementation for User Story 2

- [x] T025 [US2] Create UpdateProfileForm struct with validation in src/handlers/settings.rs
- [x] T026 [US2] Implement POST /settings/profile handler in src/handlers/settings.rs
- [x] T027 [US2] Add input sanitization to reject HTML/script tags in profile fields (RISK-004-002)
- [x] T028 [US2] Add CSRF token to profile edit form in templates/settings/profile.html
- [x] T029 [US2] Implement form validation error display in templates/settings/profile.html
- [x] T030 [US2] Add success/error flash message handling
- [x] T031 [US2] Log profile update to AuditService (RISK-004-005)

**Checkpoint**: User Story 2 complete - users can edit their profile ✅

---

## Phase 5: User Story 3 - Manage Email Address (Priority: P2) ✅

**Goal**: Allow users to change their email with verification

**Independent Test**: Request email change, verify email sent, confirm change via link

### Implementation for User Story 3

- [x] T032 [US3] Create ChangeEmailForm struct in src/handlers/settings.rs
- [x] T033 [US3] Create account settings template in templates/settings/account.html
- [x] T034 [US3] Implement GET /settings/account handler in src/handlers/settings.rs
- [x] T035 [US3] Implement POST /settings/account/email handler calling existing API in src/handlers/settings.rs
- [x] T036 [US3] Add generic response messaging to prevent email enumeration (RISK-004-007)
- [x] T037 [US3] Add CSRF token to email change form
- [x] T038 [US3] Add rate limiting for email change endpoint (RISK-004-001)
- [x] T039 [US3] Log email change request to AuditService (RISK-004-005)

**Checkpoint**: User Story 3 complete - users can change their email ✅

---

## Phase 6: User Story 4 - Change Password (Priority: P2) ✅

**Goal**: Allow users to change password with session invalidation

**Independent Test**: Change password, verify logout on other devices, verify login with new password

### Implementation for User Story 4

- [x] T040 [US4] Create security settings template in templates/settings/security.html
- [x] T041 [US4] Implement GET /settings/security handler in src/handlers/settings.rs
- [x] T042 [US4] Create ChangePasswordForm struct with length-based validation (12+ chars per NIST SP 800-63B) in src/handlers/settings.rs
- [x] T043 [US4] Implement POST /settings/security/password handler in src/handlers/settings.rs
- [x] T044 [US4] Implement invalidate_other_sessions(user_id, current_session_id) in src/services/session_service.rs (FR-017)
- [x] T045 [US4] Call session invalidation after successful password change
- [x] T046 [US4] Add rate limiting middleware for password change endpoint (RISK-004-001)
- [x] T047 [US4] Add CSRF token to password change form
- [x] T048 [US4] Log password change to AuditService (RISK-004-005)

**Checkpoint**: User Story 4 complete - users can change password with session management ✅

---

## Phase 7: User Story 5 - Manage Two-Factor Authentication (Priority: P2) ✅

**Goal**: Allow users to enable/disable 2FA, view recovery codes, and regenerate recovery codes

**Independent Test**: Enable 2FA via QR code, verify login requires code, regenerate recovery codes, disable 2FA

### Implementation for User Story 5

- [x] T049 [US5] Add 2FA status section to templates/settings/security.html
- [x] T050 [US5] Implement GET /settings/security/2fa/setup handler returning QR code in src/handlers/settings.rs
- [x] T051 [US5] Create 2FA setup modal/page in templates/settings/2fa_setup.html
- [x] T052 [US5] Implement POST /settings/security/2fa/enable handler in src/handlers/settings.rs
- [x] T053 [US5] Create recovery codes display template in templates/settings/2fa_recovery.html
- [x] T054 [US5] Implement POST /settings/security/2fa/disable handler in src/handlers/settings.rs
- [x] T055 [US5] Implement POST /settings/security/2fa/recovery handler for code regeneration in src/handlers/settings.rs
- [x] T056 [US5] Add rate limiting for 2FA enable/disable/recovery endpoints (RISK-004-001, RISK-004-003)
- [x] T057 [US5] Verify TOTP secret never logged or returned except during setup (RISK-004-004)
- [x] T058 [US5] Log 2FA enable/disable/recovery to AuditService (RISK-004-005)

**Checkpoint**: User Story 5 complete - users can manage 2FA ✅

---

## Phase 8: User Story 6 - Manage Active Sessions (Priority: P3) ✅

**Goal**: Display active sessions and allow revocation

**Independent Test**: View session list, revoke a session, verify revoked session cannot access app

### Implementation for User Story 6

- [x] T059 [US6] Add sessions list section to templates/settings/security.html
- [x] T060 [US6] Create SessionInfo display struct in src/handlers/settings.rs
- [x] T061 [US6] Implement sessions list rendering with current session highlighted
- [x] T062 [US6] Implement POST /settings/security/sessions/{id}/revoke handler in src/handlers/settings.rs
- [x] T063 [US6] Verify session.user_id == auth.user_id before revocation (RISK-004-010)
- [x] T064 [US6] Prevent revocation of current session (FR-012)
- [x] T065 [US6] Implement session limit per user (10 max) in src/services/session_service.rs (RISK-004-009)
- [x] T066 [US6] Log session revocation to AuditService (RISK-004-005)

**Checkpoint**: User Story 6 complete - users can manage sessions ✅

---

## Phase 9: User Story 7 - Request Account Deletion (Priority: P3) ✅

**Goal**: Allow users to request account deletion with content handling choice

**Independent Test**: Request deletion, verify grace period shown, cancel deletion

### Implementation for User Story 7

- [x] T067 [US7] Add account deletion section to templates/settings/account.html
- [x] T068 [US7] Create deletion confirmation modal in templates/components/deletion_modal.html
- [x] T069 [US7] Create DeleteAccountForm struct with content_choice in src/handlers/settings.rs
- [x] T070 [US7] Implement POST /settings/account/delete handler in src/handlers/settings.rs
- [x] T071 [US7] Add deletion pending banner to settings layout when deletion_requested_at is set
- [x] T072 [US7] Implement POST /settings/account/cancel-deletion handler in src/handlers/settings.rs
- [x] T073 [US7] Add focus trap to deletion modal for accessibility (research.md)
- [x] T074 [US7] Require password re-authentication for deletion (FR-006)
- [x] T075 [US7] Add rate limiting for deletion endpoint (RISK-004-001)
- [x] T076 [US7] Log deletion request/cancellation to AuditService (RISK-004-005)

**Checkpoint**: User Story 7 complete - users can manage account deletion ✅

---

## Phase 10: Polish & Cross-Cutting Concerns ✅

**Purpose**: Improvements that affect multiple user stories, integration testing, and final validation

### Accessibility & Polish

- [x] T077 [P] Add skip link to main content in templates/settings/_layout.html (WCAG 2.1 AA)
- [x] T078 [P] Ensure all form inputs have visible focus states (accessibility)
- [x] T079 [P] Add aria-describedby for all form error messages
- [x] T080 Verify all templates use Askama auto-escaping (RISK-004-002)
- [x] T081 [P] Add mobile-responsive styles to settings templates

### Integration Tests (Constitution Code Quality Gates)

- [x] T082 Create settings handler test module in tests/settings_test.rs
- [x] T083 [P] Add integration tests for profile view/edit endpoints (US1, US2)
- [x] T084 [P] Add integration tests for email change endpoint (US3)
- [x] T085 [P] Add integration tests for password change with session invalidation (US4)
- [x] T086 [P] Add integration tests for 2FA enable/disable/recovery (US5)
- [x] T087 [P] Add integration tests for session management (US6)
- [x] T088 [P] Add integration tests for account deletion flow (US7)
- [x] T089 Add security tests for rate limiting and CSRF protection

### Validation

- [x] T090 Run quickstart.md validation scenarios
- [x] T091 Run cargo clippy and fix any warnings
- [x] T092 Update CLAUDE.md with feature context

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-9)**: All depend on Foundational phase completion
  - User stories can proceed in priority order (P1 → P2 → P3)
  - US1 and US2 share templates - implement US1 first
  - US3-US7 are largely independent
- **Polish (Phase 10)**: Depends on all user stories being complete
  - Integration tests can run after each user story is complete

### User Story Dependencies

- **User Story 1 (P1)**: Foundation only - provides base profile viewing
- **User Story 2 (P1)**: Depends on US1 (extends profile.html) - adds editing
- **User Story 3 (P2)**: Foundation only - independent (account.html)
- **User Story 4 (P2)**: Foundation only - independent (security.html)
- **User Story 5 (P2)**: Foundation only - extends security.html
- **User Story 6 (P3)**: Foundation only - extends security.html
- **User Story 7 (P3)**: Depends on US3 (shares account.html) - adds deletion

### Security Risk Mitigation Order

P0 risks addressed in Foundational phase:
- RISK-004-002: CSP header (T011)

P0/P1 risks addressed during implementation:
- RISK-004-001: Rate limiting (T038, T046, T056, T075)
- RISK-004-002: Input sanitization (T027)
- RISK-004-003: 2FA rate limiting (T056)
- RISK-004-004: TOTP secret protection (T009, T057)
- RISK-004-005: Audit logging (T014-T016, T031, T039, T048, T058, T066, T076)

### Parallel Opportunities

```bash
# Phase 1 - All can run in parallel:
T002, T003, T004 (template structure)

# Phase 2 - Security verifications in parallel:
T008, T009, T010, T012 (different files)

# Phase 4 - Form and validation in parallel:
T025, T028 (handler struct vs template)

# Phase 10 - Integration tests in parallel:
T083, T084, T085, T086, T087, T088 (independent test files)
```

---

## Implementation Strategy

### MVP First (User Stories 1-2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (View Profile)
4. Complete Phase 4: User Story 2 (Edit Profile)
5. **STOP and VALIDATE**: Test profile view/edit independently
6. Deploy/demo if ready

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 + US2 → Profile management (MVP!)
3. Add US3 + US4 → Email/password management
4. Add US5 → 2FA management
5. Add US6 + US7 → Session/deletion management
6. Each story adds value without breaking previous stories

### Risk-First Strategy

Address P0 security risks (RISK-004-001, RISK-004-002) in Foundational phase before any user-facing features. This ensures security controls are in place from the start.

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- OSK risks referenced as (RISK-004-XXX) for traceability
- All handlers require AuthUser middleware (RISK-004-E02)
- All security-sensitive operations logged via AuditService (RISK-004-005)
- Password validation uses length-based approach (12+ chars) per NIST SP 800-63B
- Rate limiting applied to all sensitive endpoints (email, password, 2FA, deletion)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Existing API endpoints in src/api/account.rs are reused - handlers call service layer
- Integration tests in Phase 10 validate all user stories per Constitution Code Quality Gates
