# Implementation Plan: ABAC Authorization System

**Branch**: `005-abac-authorization` | **Date**: 2025-12-30 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/005-abac-authorization/spec.md`

## Summary

Implement a comprehensive Attribute-Based Access Control (ABAC) system for Oppskrift that enables flexible resource sharing. The system supports:
- Owner full control over resources (recipes, books)
- Public, private, and followers-only visibility levels
- Sharing with specific users, groups, and federated instances
- Collaborative book editing where multiple users can contribute recipes while maintaining individual ownership
- Permission levels: view, edit, contributor (books only)

Technical approach: Extend the existing visibility-based authorization with a permission entity model, integrate with the existing Axum middleware pattern, and leverage SQLx for compile-time checked queries.

## Technical Context

**Language/Version**: Rust 2021 edition (1.75+)
**Primary Dependencies**: Axum 0.8, SQLx 0.8, tokio 1.x, tower-http 0.6, validator 0.20, activitypub_federation 0.6
**Storage**: PostgreSQL 15+ (via SQLx with offline mode)
**Testing**: axum-test 18.x with tokio-test, integration tests in /tests/
**Target Platform**: Linux server, containerized deployment (Docker/Podman)
**Project Type**: Web application (server-rendered HTML + REST API)
**Performance Goals**: Authorization decisions <50ms for 95% of requests (per SC-001)
**Constraints**: Groups support 1000+ members without degradation (per SC-005)
**Scale/Scope**: Federation-ready, multi-instance deployment capable

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Federation First | PASS | Instance-level sharing aligns with ActivityPub; Permission objects can be federated |
| II. Security by Default | PASS | 404 responses hide resource existence; permission checks at service layer; audit logging required |
| III. Standards Compliance | PASS | REST APIs; existing patterns maintained |
| IV. User Experience | PASS | Permission management UI specified; share in <30s target |
| V. Maintainability | PASS | Extends existing service layer pattern; modular permission system |
| VI. Open Source Ethos | PASS | No proprietary dependencies required |
| VII. Security Integration | PENDING | Must run /osk-analyze before implementation |

**Gate Status**: PASS (pending OSK integration during implementation phase)

## Project Structure

### Documentation (this feature)

```text
specs/005-abac-authorization/
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
├── models/
│   ├── permission.rs        # NEW: Permission, PermissionLevel, SubjectType
│   ├── group.rs             # NEW: Group, GroupMember
│   ├── visibility.rs        # MODIFY: Add FollowersOnly variant
│   ├── recipe.rs            # EXISTING
│   ├── recipe_book.rs       # EXISTING
│   └── mod.rs               # UPDATE: export new modules

├── services/
│   ├── permission_service.rs  # NEW: Permission CRUD, check logic
│   ├── group_service.rs       # NEW: Group management
│   ├── recipe_service.rs      # MODIFY: Integrate permission checks
│   ├── book_service.rs        # MODIFY: Integrate permission checks, contributor logic
│   └── mod.rs                 # UPDATE: export new modules

├── api/
│   ├── permissions.rs         # NEW: Permission management endpoints
│   ├── groups.rs              # NEW: Group management endpoints
│   ├── recipes.rs             # MODIFY: Update authorization
│   ├── books.rs               # MODIFY: Update authorization, contributor endpoints
│   └── mod.rs                 # UPDATE: register new routes

├── handlers/
│   ├── permissions.rs         # NEW: Permission UI pages
│   ├── groups.rs              # NEW: Group UI pages
│   └── mod.rs                 # UPDATE: register new handlers

└── core/
    └── mod.rs                 # EXISTING

templates/
├── permissions/               # NEW: Permission management templates
│   ├── manage.html           # Share settings page
│   └── components/           # Permission list, user search
├── groups/                    # NEW: Group management templates
│   ├── list.html
│   ├── detail.html
│   └── create.html
└── components/
    └── share_button.html      # NEW: Quick share component

migrations/
├── YYYYMMDDHHMMSS_add_visibility_followers_only.sql
├── YYYYMMDDHHMMSS_create_groups_table.sql
├── YYYYMMDDHHMMSS_create_group_members_table.sql
├── YYYYMMDDHHMMSS_create_permissions_table.sql
├── YYYYMMDDHHMMSS_create_book_contributions_table.sql
└── YYYYMMDDHHMMSS_create_permission_audit_log.sql

tests/
├── permissions_test.rs        # NEW: Permission integration tests
├── groups_test.rs             # NEW: Group integration tests
├── authorization_test.rs      # NEW: ABAC authorization tests
└── common/
    └── mod.rs                 # UPDATE: Add permission/group helpers
```

**Structure Decision**: Extends existing single-project structure. New modules follow established patterns (models → services → api/handlers). No new projects required.

## Complexity Tracking

> No constitution violations requiring justification. The permission system adds necessary complexity for the core sharing feature.

| Aspect | Justification |
|--------|---------------|
| Permission caching (FR-019) | Required for performance target; uses existing patterns |
| Multiple permission paths (FR-008) | Core ABAC requirement; well-defined evaluation order |
| Book contribution tracking (FR-022-26) | Unique feature differentiator for collaborative use case |
