# Implementation Plan: UI Parity

**Branch**: `006-ui-parity` | **Date**: 2026-01-01 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/006-ui-parity/spec.md`

## Summary

Close the gap between existing API endpoints and UI by adding HTML handlers and templates for: sessions management, security events viewer, federation toggle, data export, book contributions management, and followers/following lists. This is primarily a UI/template-focused feature leveraging existing API infrastructure.

## Technical Context

**Language/Version**: Rust 2021 edition (1.75+)
**Primary Dependencies**: Axum 0.8, Askama 0.15, SQLx 0.8, HTMX (vendored in static/)
**Storage**: PostgreSQL 15+ (existing tables: sessions, security_events, follows, book_contributions)
**Testing**: cargo test, axum-test for integration tests
**Target Platform**: Linux server (web application)
**Project Type**: Web application with server-rendered HTML + HTMX
**Performance Goals**: Page loads under 2 seconds, 100 users concurrent
**Constraints**: No JavaScript frameworks, HTMX for interactivity, WCAG 2.1 AA accessibility
**Scale/Scope**: ~10 new templates, ~6 handler modules, ~200-400 LOC per module

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Federation First | ✅ PASS | Federation toggle respects ActivityPub protocol |
| II. Security by Default | ✅ PASS | All endpoints require authentication; session management enables security awareness |
| III. Standards Compliance | ✅ PASS | JSON export follows JSON-LD patterns; REST conventions |
| IV. User Experience | ✅ PASS | WCAG 2.1 AA target; mobile-first; <3 clicks for core actions |
| V. Maintainability | ✅ PASS | Follows existing module patterns; clear template structure |
| VI. Open Source Ethos | ✅ PASS | No proprietary dependencies |
| VII. Security Integration | ⚠ DEFER | Security analysis to be performed via /osk-analyze post-plan |

**Gate Result**: PASS - All blocking principles satisfied. Security integration deferred to OSK workflow.

## Project Structure

### Documentation (this feature)

```text
specs/006-ui-parity/
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
├── handlers/
│   ├── settings.rs      # EXTEND: sessions page, security events, federation toggle, data export
│   ├── books.rs         # EXTEND: contributions management section
│   └── users.rs         # EXTEND: followers/following list pages
├── api/
│   └── users.rs         # EXTEND: followers/following list endpoints (if needed)
├── models/
│   └── (existing)       # Session, SecurityEvent, Follow, BookContribution - no changes
└── services/
    └── (existing)       # SessionService, FollowService, etc. - minimal changes

templates/
├── settings/
│   ├── sessions.html         # EXTEND: full sessions table with revoke
│   ├── security_events.html  # NEW: security events list with pagination
│   └── privacy.html          # NEW: federation toggle + data export
├── books/
│   └── view.html             # EXTEND: contributions section
└── users/
    ├── followers.html        # NEW: followers list page
    └── following.html        # NEW: following list page

tests/
├── ui_parity_test.rs         # NEW: integration tests for new UI endpoints
```

**Structure Decision**: Extending existing handler modules rather than creating new ones. Templates follow established `/templates/{module}/{page}.html` pattern.

## Complexity Tracking

No constitution violations requiring justification. Feature uses existing patterns.

---

## Post-Design Constitution Re-Check

*Re-evaluated after Phase 1 design artifacts completed.*

| Principle | Status | Post-Design Notes |
|-----------|--------|-------------------|
| I. Federation First | ✅ PASS | Federation toggle properly gates WebFinger/AP endpoints |
| II. Security by Default | ✅ PASS | All new handlers use AuthUser middleware; CSRF on all forms |
| III. Standards Compliance | ✅ PASS | JSON-LD export format; semantic HTML in templates |
| IV. User Experience | ✅ PASS | Templates include ARIA labels; pagination for large lists |
| V. Maintainability | ✅ PASS | Follows existing patterns; no new dependencies added |
| VI. Open Source Ethos | ✅ PASS | No changes to licensing or dependencies |
| VII. Security Integration | ✅ READY | Security considerations documented in research.md |

**Post-Design Gate Result**: PASS - All principles satisfied. Ready for task generation.

---

## Generated Artifacts

| Artifact | Path | Status |
|----------|------|--------|
| Research | `specs/006-ui-parity/research.md` | ✅ Complete |
| Data Model | `specs/006-ui-parity/data-model.md` | ✅ Complete |
| Handler Contracts | `specs/006-ui-parity/contracts/handlers.md` | ✅ Complete |
| Quickstart | `specs/006-ui-parity/quickstart.md` | ✅ Complete |

---

## Security Analysis (Deferred)

**Status**: OSK workflow deferred to implementation phase

**Rationale**: This feature exposes existing APIs through UI templates. Core security controls (authentication, authorization, CSRF) are already implemented. New attack surface is minimal.

**Required Before Merge**:
- [ ] Run `/osk-analyze` focusing on: session revocation, data export rate limiting, contribution authorization
- [ ] Document findings in `.osk/specs/006-ui-parity/threats.md`
- [ ] Add any resulting security tasks to Phase 9 (Polish)

**Key Security Considerations** (from research.md):
1. Session revocation: Prevent current session self-revoke ✓ (handled in API)
2. Security events: User can only see own events ✓ (WHERE user_id = $1)
3. Data export: Rate limit to 1 per hour ✓ (T028)
4. Contribution rejection: Sanitize rejection reason ✓ (prevent XSS)
5. Federation toggle: Clear warning about discoverability ✓ (T022)

---

## Next Steps

Run `/speckit.tasks` to generate implementation tasks from this plan.
