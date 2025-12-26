# Research: Recipe Creation and Sharing

**Feature**: 001-recipe-sharing
**Date**: 2025-12-25
**Status**: Complete

## Technology Stack Decisions

### Backend Framework

**Decision**: Rust + Axum 0.8

**Rationale**:
- Axum surpassed Actix-web in community adoption (2023 Rust Developer Survey)
- Backed by Tokio team with strong stability guarantees (Hyper 1.0: no breaking changes for 3 years)
- Follows standard Rust patterns without additional abstractions
- Uses Tower middleware for composability
- Better documentation and developer experience than Actix

**Alternatives considered**:
- **Actix-web**: Higher raw throughput but more complex (Actor model), steeper learning curve
- **Elixir/Phoenix**: Excellent for real-time but requires learning new language/ecosystem
- **Go + Fiber**: Simpler but fewer ActivityPub libraries available

### ActivityPub Library

**Decision**: activitypub-federation-rust 0.6.3+

**Rationale**:
- De facto standard for Rust ActivityPub (extracted from Lemmy)
- Active maintenance (24 contributors, 473 stars, beta releases through Oct 2025)
- Framework-agnostic (works with Axum)
- Handles HTTP signatures, fetching/sending/receiving
- Flexible: define your own entities with Serde

**Alternatives considered**:
- No competing Rust ActivityPub libraries exist with similar maturity

### Database ORM

**Decision**: SQLx

**Rationale**:
- Compile-time checked SQL queries
- Async-native (works well with Axum/Tokio)
- No runtime overhead from ORM abstractions
- Direct SQL control for complex queries
- Good PostgreSQL support including JSON types

**Alternatives considered**:
- **Diesel**: Used by Lemmy, more ORM-like, sync by default (async adapter available)
- **SeaORM**: Higher-level abstraction, less mature than SQLx

### Frontend Approach

**Decision**: Server-rendered HTML + HTMX

**Rationale**:
- Progressive enhancement: works without JavaScript (constitution requirement)
- Simpler architecture: single Rust codebase serves HTML
- Reduced client-side complexity
- Better accessibility by default
- Faster initial page loads

**Alternatives considered**:
- **React/Next.js**: Heavier client bundle, more complex build
- **SvelteKit**: Good but adds separate build pipeline
- **Phoenix LiveView**: Requires Elixir backend

### Template Engine

**Decision**: Askama (or Tera)

**Rationale**:
- Askama: Compile-time template checking, type-safe, fast
- Integrates well with Axum
- Jinja2-like syntax (familiar)

**Alternatives considered**:
- **Tera**: Runtime templates, more flexible but slower
- **Maud**: Rust macros for HTML, steeper learning curve

### Image Processing

**Decision**: image-rs + S3-compatible storage

**Rationale**:
- image-rs: Pure Rust, no external dependencies
- S3-compatible: Works with MinIO for self-hosting, AWS S3, or others
- Constitution requires configurable storage backends

**Alternatives considered**:
- **ImageMagick bindings**: More features but external dependency
- **Local filesystem only**: Doesn't scale, but supported as fallback

### Testing Framework

**Decision**: Rust built-in + tokio-test + sqlx-test

**Rationale**:
- Native Rust testing with `#[test]` and `#[tokio::test]`
- Unit tests inline in each file with `#[cfg(test)] mod tests { ... }` (Rust convention)
- Integration tests in `tests/` folder at crate root
- sqlx provides test fixtures and database isolation
- No additional test framework dependencies needed

**Alternatives considered**:
- **rstest**: Parameterized tests (may add if needed)

## Schema.org Recipe Mapping

**Decision**: Custom Rust structs with serde serialization to JSON-LD

Based on [Schema.org Recipe](https://schema.org/Recipe), key fields to implement:

| Schema.org Property | Rust Field | Type |
|---------------------|------------|------|
| name | title | String |
| description | description | String |
| recipeIngredient | ingredients | Vec<Ingredient> |
| recipeInstructions | instructions | Vec<InstructionStep> |
| prepTime | prep_time_minutes | Option<u32> |
| cookTime | cook_time_minutes | Option<u32> |
| totalTime | (computed) | Duration |
| recipeYield | servings | Option<String> |
| image | images | Vec<Image> |
| author | author | User reference |
| datePublished | created_at | DateTime |
| dateModified | updated_at | DateTime |

**Rationale**: Direct mapping ensures interoperability with recipe aggregators and SEO.

## ActivityPub Object Mapping

Recipes and Recipe Books map to ActivityPub objects:

| Entity | ActivityPub Type | Notes |
|--------|------------------|-------|
| Recipe | Article or Note with Recipe attachment | Custom extension for recipe data |
| RecipeBook | Collection | OrderedCollection of recipe references |
| User | Person/Actor | Standard ActivityPub actor |
| Share | Announce | Standard ActivityPub activity |
| Create Recipe | Create | Standard ActivityPub activity |

**Rationale**: Follow established patterns from Lemmy and Mastodon.

## Metric/Imperial Conversion

**Decision**: Store in metric, convert on display

**Implementation**:
- Store quantities as decimal with metric unit (g, ml, cm, etc.)
- User preference stored in profile
- Conversion library: `uom` (units of measurement) crate
- Display formatting respects locale

## Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| API response p95 | <200ms | Prometheus metrics |
| Page load (3G) | <3s | Lighthouse |
| Concurrent users | 1000 | Load testing (k6) |
| Feed update latency | <30s | End-to-end test |

## Security Considerations

| Requirement | Implementation |
|-------------|----------------|
| Authentication | OAuth 2.0 via separate auth service |
| Authorization | Per-resource ownership checks |
| Input validation | Validator crate + custom rules |
| Rate limiting | tower-governor middleware |
| HTTPS | Mandatory (reverse proxy handles TLS) |
| Image uploads | Size limits, type validation, virus scan optional |

## Dependencies Summary

```toml
# Cargo.toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "chrono", "uuid"] }
activitypub_federation = "0.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
askama = "0.12"
askama_axum = "0.4"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
validator = { version = "0.18", features = ["derive"] }
thiserror = "2"
anyhow = "1"
image = "0.25"
aws-sdk-s3 = "1"
```

**Static assets**:
- HTMX 2.x (vendored in `static/js/`)
- Tailwind CSS via standalone CLI (no Node required)

### CSS Framework

**Decision**: Tailwind CSS with standalone CLI

**Rationale**:
- Standalone CLI binary - no Node.js required in build pipeline
- Scales well for theming and design system consistency
- Built-in dark mode support (`class` strategy for user toggle)
- CSS custom properties for runtime instance theming
- Atomic CSS output is small (~10-30KB purged)
- Large ecosystem of patterns and examples for UI components

**Alternatives considered**:
- **Vanilla CSS**: Doesn't scale for theming, harder to maintain consistency
- **Panda CSS**: Requires Node.js
- **UnoCSS**: Requires Node.js

## Reference Projects

- **Lemmy**: Production Rust ActivityPub (study federation patterns)
- **Hatsu**: Lightweight Rust ActivityPub bridge
- **Mastodon**: Reference ActivityPub implementation (Ruby, but good spec adherence)

## Open Questions (Resolved)

| Question | Resolution |
|----------|------------|
| Backend language? | Rust (user familiarity) |
| Web framework? | Axum 0.8 (modern, well-maintained) |
| Frontend approach? | Server-rendered + HTMX |
| ActivityPub library? | activitypub-federation-rust |
| Database ORM? | SQLx (compile-time checked) |
