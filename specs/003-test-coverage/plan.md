# Implementation Plan: Comprehensive Test Coverage

**Branch**: `003-test-coverage` | **Date**: 2025-12-28 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-test-coverage/spec.md`

## Summary

Implement comprehensive unit and integration tests for the Oppskrift codebase, covering all 73 source files. Currently 41 files have unit tests (56%), leaving 32 files without test coverage. The plan adds missing unit tests, expands integration tests beyond auth, and establishes test helpers for consistent test patterns.

## Technical Context

**Language/Version**: Rust 1.75+
**Primary Dependencies**: Axum 0.8, SQLx 0.8, tokio, activitypub_federation 0.6
**Storage**: PostgreSQL 15+ (via SQLx)
**Testing**: cargo test, axum-test for integration tests
**Target Platform**: Linux server
**Project Type**: Single project (web application)
**Performance Goals**: Test suite completes in under 5 minutes
**Constraints**: Tests must be isolated, no flakiness, clear failure messages
**Scale/Scope**: 73 source files, 32 missing tests, ~150+ new test cases

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Federation First | ✅ Pass | Tests will cover ActivityPub endpoints |
| II. Security by Default | ✅ Pass | Security-critical auth flows already tested |
| III. Standards Compliance | ✅ Pass | Tests validate API contracts |
| IV. User Experience | N/A | Testing is developer-focused |
| V. Maintainability | ✅ Pass | Tests improve maintainability |
| VI. Open Source Ethos | ✅ Pass | Tests enable confident contributions |
| VII. Security Integration | ⚠️ Deferred | No new security features; test existing |

**Gate Status**: PASS - Testing feature is infrastructure, not new functionality requiring security design.

## Project Structure

### Documentation (this feature)

```text
specs/003-test-coverage/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output (minimal - no new entities)
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (N/A - testing infrastructure)
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
src/
├── api/                 # API endpoints (12 files, 0 have tests)
│   ├── auth.rs
│   ├── account.rs
│   ├── recipes.rs
│   ├── books.rs
│   ├── users.rs
│   ├── feeds.rs
│   ├── social.rs
│   ├── activitypub.rs
│   ├── webfinger.rs
│   ├── oembed.rs        # Has tests
│   ├── openapi.rs
│   └── middleware/
├── core/                # Core utilities (11 files, 8 have tests)
│   ├── audit.rs         # Has tests
│   ├── config.rs        # Has tests
│   ├── db.rs            # Has tests
│   ├── error.rs         # Has tests
│   ├── pagination.rs    # Has tests
│   ├── request_id.rs    # Has tests
│   ├── schema_org.rs    # Has tests
│   ├── storage.rs       # Has tests
│   ├── crypto.rs        # Needs tests
│   ├── seeds/           # Needs tests
│   └── activitypub/     # Partially has tests
├── handlers/            # HTML handlers (7 files, 0 have tests)
│   ├── auth.rs
│   ├── books.rs
│   ├── feed.rs
│   ├── legal.rs
│   ├── recipes.rs
│   └── users.rs
├── jobs/                # Background jobs (2 files, 1 has tests)
│   ├── cleanup.rs       # Has tests
│   └── mod.rs
├── models/              # Data models (17 files, 15 have tests)
│   └── [most have inline tests]
├── services/            # Business logic (12 files, 12 have tests)
│   └── [all have inline tests]
├── lib.rs               # Needs tests
└── main.rs              # Entry point (no tests needed)

tests/
├── auth_test.rs         # Existing auth integration tests (31 tests)
├── common/
│   └── mod.rs           # Test helpers
├── api_test.rs          # NEW: API endpoint tests
├── recipes_test.rs      # NEW: Recipe CRUD integration tests
├── books_test.rs        # NEW: Book management tests
├── social_test.rs       # NEW: Follow/feed tests
└── federation_test.rs   # NEW: ActivityPub tests
```

**Structure Decision**: Single project structure. Tests follow Rust conventions - unit tests inline with source (`#[cfg(test)] mod tests`), integration tests in `tests/` directory.

## Complexity Tracking

No complexity violations. Test coverage is a constitutional requirement (Section V: Maintainability).

## Files Requiring Tests

### Priority 1: API Endpoints (12 files → 12 integration tests)

| File | Current State | Test Strategy |
|------|---------------|---------------|
| `src/api/auth.rs` | No unit tests | Integration tests exist in auth_test.rs |
| `src/api/account.rs` | No unit tests | Add to auth_test.rs |
| `src/api/recipes.rs` | No tests | New recipes_test.rs |
| `src/api/books.rs` | No tests | New books_test.rs |
| `src/api/users.rs` | No tests | New users_test.rs |
| `src/api/feeds.rs` | No tests | New social_test.rs |
| `src/api/social.rs` | No tests | New social_test.rs |
| `src/api/activitypub.rs` | No tests | New federation_test.rs |
| `src/api/webfinger.rs` | No tests | New federation_test.rs |
| `src/api/openapi.rs` | No tests | Unit test for spec generation |
| `src/api/oembed.rs` | Has tests | ✅ Complete |
| `src/api/middleware/` | Has tests | ✅ Complete |

### Priority 2: Handlers (7 files → unit tests)

| File | Test Strategy |
|------|---------------|
| `src/handlers/auth.rs` | Unit tests for template rendering |
| `src/handlers/books.rs` | Unit tests for template rendering |
| `src/handlers/feed.rs` | Unit tests for template rendering |
| `src/handlers/legal.rs` | Unit tests for template rendering |
| `src/handlers/recipes.rs` | Unit tests for template rendering |
| `src/handlers/users.rs` | Unit tests for template rendering |

### Priority 3: Core Utilities (3 files missing tests)

| File | Test Strategy |
|------|---------------|
| `src/core/crypto.rs` | Unit tests for key generation |
| `src/core/seeds/*.rs` | Unit tests for seed data |
| `src/lib.rs` | Integration test for app creation |

### Already Complete (41 files)

- All 12 service modules have unit tests
- 15 of 17 model modules have unit tests
- 8 of 11 core modules have unit tests
- Existing auth integration tests (31 tests)
