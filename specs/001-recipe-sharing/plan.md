# Implementation Plan: Recipe Creation and Sharing

**Branch**: `001-recipe-sharing` | **Date**: 2025-12-25 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-recipe-sharing/spec.md`

## Summary

Enable users to create, organize, and share recipes through a federated social platform. Users can create detailed recipes with ingredients and instructions, organize them into recipe books (collections), and share content with followers across federated instances via ActivityPub. All recipe data conforms to Schema.org Recipe vocabulary for interoperability.

## Technical Context

**Language/Version**: Rust 1.75+, HTML/CSS + HTMX (server-rendered)
**Primary Dependencies**: Axum 0.8, SQLx, activitypub-federation-rust 0.6, Askama, Tailwind CSS (standalone CLI)
**Storage**: PostgreSQL 15+ (per constitution mandate), S3-compatible for images
**Testing**: Rust built-in tests + tokio-test + sqlx-test
**Target Platform**: Linux server (Docker containers), Web browsers (responsive, mobile-first)
**Project Type**: Single Rust application (server-rendered HTML + JSON API)
**Performance Goals**: 1000 concurrent users, <3s page load on 3G, activity feed updates within 30 seconds
**Constraints**: <200ms API response p95, WCAG 2.1 AAA target (AA minimum), works without JavaScript for critical paths
**Scale/Scope**: Initial target 1000 concurrent users, 50 ingredients/30 steps/10 images per recipe limit

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Requirement | Status |
|-----------|-------------|--------|
| I. Federation First | Recipes/books as ActivityPub objects, WebFinger discovery | ✅ Planned (FR-022, FR-023, FR-024) |
| II. Security by Default | OAuth 2.0/OIDC auth, TLS 1.3, input validation, rate limiting | ✅ Planned (assumed auth system) |
| III. Standards Compliance | Schema.org Recipe, JSON-LD, REST APIs, ISO 8601 dates | ✅ Planned (FR-006) |
| IV. User Experience | WCAG 2.1 AAA target (AA min), keyboard nav, screen reader, ARIA, skip links, focus indicators, alt text, high contrast | ✅ Planned (SC-009 to SC-014) |
| V. Maintainability | Docker deployment, env config, PostgreSQL, modular code | ✅ Planned (technical context) |
| VI. Open Source Ethos | OSI license, self-host docs, no proprietary deps | ✅ Planned |

**Gate Status**: ✅ PASS - All constitutional requirements addressed in feature spec.

## Project Structure

### Documentation (this feature)

```text
specs/001-recipe-sharing/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (OpenAPI specs)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
src/
├── models/              # Recipe, RecipeBook, Ingredient, User, Activity
│                        # (unit tests inline with #[cfg(test)])
├── services/            # RecipeService, BookService, ActivityService, FederationService
├── api/                 # REST endpoints, ActivityPub handlers
├── handlers/            # HTML page handlers (serve templates)
└── lib/                 # Shared utilities, Schema.org serialization
    ├── seeds/           # Development seed data (users.rs, recipes.rs)
    └── seed.rs          # Seed runner with --seed CLI flag

templates/               # Askama templates (.html)
├── layouts/             # Base layouts (base.html, etc.)
├── recipes/             # Recipe views (create, view, edit, list)
├── books/               # Book views
├── feed/                # Activity feed
└── components/          # Reusable partials (recipe_card, ingredient_list)

static/                  # Served directly by Axum
├── css/
│   ├── input.css        # Tailwind source (directives + custom)
│   └── main.css         # Generated output (gitignored)
└── js/
    └── htmx.min.js      # HTMX (vendored)

tailwind.config.js       # Tailwind configuration (themes, colors)
tailwindcss              # Standalone CLI binary (gitignored)

tests/                   # Integration tests only (Rust convention)
├── api/                 # API endpoint integration tests
├── federation/          # ActivityPub federation tests
└── e2e/                 # End-to-end browser tests (Playwright)
```

**Structure Decision**: Single Rust application serving both API and HTML. Askama compiles templates at build time for type-safe, fast rendering. HTMX provides interactivity without a JavaScript framework. This enables:
- Single deployment artifact (one binary)
- Progressive enhancement (works without JS)
- Type-safe templates (compile-time errors)
- Clear API contracts for third-party integrations

## Complexity Tracking

> No constitutional violations requiring justification.

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Single application | Yes | Server-rendered HTML + JSON API in one Rust binary; simpler deployment |
| ActivityPub integration | Required | Constitution Principle I mandates federation |
| Schema.org compliance | Required | Constitution Principle III mandates standards |
