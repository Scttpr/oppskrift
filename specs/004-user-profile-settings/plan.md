# Implementation Plan: User Profile & Settings Management

**Branch**: `004-user-profile-settings` | **Date**: 2025-12-29 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/004-user-profile-settings/spec.md`

## Summary

This feature provides authenticated users with a comprehensive settings interface to view and manage their profile information, security settings, and account lifecycle. It leverages existing User model infrastructure and account API endpoints, adding HTML handler pages and extending functionality for account deletion with content handling choices.

## Technical Context

**Language/Version**: Rust 1.75+
**Primary Dependencies**: Axum 0.8, Askama (templates), SQLx (database), validator (input validation)
**Storage**: PostgreSQL 15+ (existing schema)
**Testing**: cargo test, axum-test for integration tests
**Target Platform**: Linux server (web application)
**Project Type**: Single project with server-rendered HTML + HTMX
**Performance Goals**: Page load < 3 seconds, settings updates < 1 second response
**Constraints**: WCAG 2.1 AA accessibility, mobile-first responsive design
**Scale/Scope**: Extends existing user management, ~10 new templates, ~5 new/modified handlers

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Federation First | ✅ Pass | Profile updates trigger ActivityPub Update activities (existing) |
| II. Security by Default | ✅ Pass | Password verification for sensitive ops, session invalidation, TOTP support, audit logging |
| III. Standards Compliance | ✅ Pass | Uses existing Schema.org, REST API patterns |
| IV. User Experience | ✅ Pass | 3-click requirement (SC-007), mobile-first, accessible forms (WCAG 2.1 AA) |
| V. Maintainability | ✅ Pass | Extends existing module structure, clear handler/template separation |
| VI. Open Source Ethos | ✅ Pass | No proprietary dependencies |
| VII. Security Integration | ✅ Pass | OSK analysis complete: `.osk/specs/004-user-profile-settings/` |

**Gate Status**: PASS

### Security Considerations

Per NIST SP 800-63B, password requirements prioritize length (12+ characters) over complexity rules. The implementation should validate minimum length with optional complexity guidance rather than mandatory mixed-case/special character requirements.

### Audit Logging

Security-sensitive operations (password change, email change, 2FA enable/disable, session revocation, account deletion) MUST be logged to a security_events table for incident response and RGPD compliance (RISK-004-005).

## Project Structure

### Documentation (this feature)

```text
specs/004-user-profile-settings/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── models/
│   └── user.rs          # Existing - User, UpdateUser, UserProfile
├── services/
│   └── user_service.rs  # Existing - profile CRUD operations
├── api/
│   └── account.rs       # Existing - account API endpoints
├── handlers/
│   ├── settings.rs      # NEW - settings page handlers
│   └── mod.rs           # Update to include settings routes
└── lib.rs

templates/
├── settings/            # NEW - settings templates
│   ├── profile.html     # Profile view/edit
│   ├── security.html    # Password, 2FA, sessions
│   ├── account.html     # Email, deletion
│   └── _nav.html        # Settings navigation partial
└── components/
    └── deletion_modal.html  # NEW - account deletion confirmation

tests/
├── settings_test.rs     # NEW - settings handler integration tests
└── common/mod.rs        # Existing test helpers
```

**Structure Decision**: Single project structure, extending existing `src/handlers/` with new settings module and `templates/settings/` directory.

## Complexity Tracking

No violations requiring justification. Feature uses existing patterns and infrastructure.
