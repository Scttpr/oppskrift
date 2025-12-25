# Codebase Audit Report

**Date**: 2025-12-25
**Codebase**: Oppskrift - Federated Recipe Sharing Platform
**Lines of Code**: ~7,292 Rust lines
**Status**: Development - Not production ready

---

## Critical Issues (Must Fix Before Production)

### 1. Invalid Cargo.toml Edition
- **Location**: `Cargo.toml:4`
- **Issue**: `edition = "2024"` is invalid
- **Fix**: Change to `edition = "2021"`

### 2. JWT Secret Defaults to Insecure Value
- **Location**: `src/api/middleware/auth.rs:87`
- **Issue**: Falls back to `"dev-secret"` if JWT_SECRET not set
- **Fix**: Panic or return error if JWT_SECRET is missing

### 3. HTTP Signature Verification Not Implemented
- **Location**: `src/api/activitypub.rs:80,95`
- **Issue**: Comments say "TODO: Verify HTTP signature" - activities accepted without verification
- **Fix**: Implement actual signature verification using public keys

### 4. Public Key is Placeholder
- **Location**: `src/api/activitypub.rs:60-61`
- **Issue**: Hardcoded placeholder string instead of real key
- **Fix**: Store/retrieve public keys per user, generate key pairs on user creation

### 5. CORS Allows All Origins
- **Location**: `src/main.rs:67-71`
- **Issue**: `CorsLayer::new().allow_origin(Any)` allows any origin
- **Fix**: Restrict to specific allowed origins via environment config

---

## High Priority Issues

### Security

| Issue | Location | Impact |
|-------|----------|--------|
| No username validation regex | `src/models/user.rs` | Invalid usernames possible |
| No HTML/XSS sanitization | `src/api/feeds.rs`, templates | Script injection in RSS/HTML |
| Rate limiting not applied | `src/main.rs` | Middleware exists but not used |
| Placeholder RSA signatures | `src/lib/activitypub/signature.rs:130` | Federation broken |
| Private keys not stored | `src/jobs/federation.rs:122` | Can't sign outgoing activities |

### Federation (ActivityPub)

| Feature | Status | Location |
|---------|--------|----------|
| HTTP Signature verification | PLACEHOLDER | `src/api/activitypub.rs` |
| HTTP Signature generation | INCOMPLETE | `src/lib/activitypub/signature.rs` |
| Public key storage | MISSING | Need DB column + service |
| Follow request handling | STUB (logs only) | `src/api/activitypub.rs:113` |
| Activity persistence | INCOMPLETE | Activities logged but not stored |
| Retry with backoff | TODO | `src/jobs/federation.rs:95` |

---

## Medium Priority Issues

| Issue | Location | Notes |
|-------|----------|-------|
| Error messages inconsistent case | `src/lib/error.rs` | Mix of lowercase/capitalized |
| BASE_URL fallback to localhost | Various | Should fail if not set in prod |
| No request ID correlation | N/A | For distributed tracing |
| Follower/following counts hardcoded to 0 | `src/api/activitypub.rs:184,207` | Need actual counts |

---

## What's Working Well

### Code Quality
- Clean module organization (`api/`, `services/`, `models/`, `handlers/`, `lib/`, `jobs/`)
- Consistent error handling with `AppError` enum and `AppResult<T>`
- No blocking calls in async contexts
- Proper use of sqlx compile-time checked queries

### Security (Implemented)
- SQL injection protected by sqlx parameterized queries
- Database errors sanitized before client response
- Ownership checks on update/delete operations
- Sensitive fields excluded from API responses

### Database
- 12 well-structured migrations
- Proper foreign key constraints with CASCADE
- Self-follow prevention constraint
- Comprehensive indexes for common queries
- Full-text search on recipes
- Automatic `updated_at` triggers

### Type Safety
- Strong enum usage (Visibility, Difficulty, MeasurementPref, ActivityType)
- Validation derives on input structs
- UUID for all IDs
- Decimal for quantities

---

## Remaining Tasks (from tasks.md)

### Phase 8 - Incomplete
- T135-T149: Accessibility validation (requires running app)
- T150: Performance optimization
- T151: Quickstart validation
- T157: Load testing with k6
- T158: UX metrics instrumentation

### Implementation Tasks Completed
- T128: Rate limiting middleware (created but not applied)
- T129: Request logging
- T130: Health check endpoint
- T131: OpenAPI documentation
- T132: Input validation
- T133: Error responses
- T134: Database indexes
- T152: CONTRIBUTING.md
- T153: RSS/Atom feeds
- T154: oEmbed endpoint
- T155: DEPLOYMENT.md
- T156: Background job queue

---

## Fix Priority Order

### Week 1 - Critical Path
1. Fix Cargo.toml edition (`"2024"` → `"2021"`)
2. Make JWT_SECRET required (fail if missing)
3. Restrict CORS to specific origins
4. Apply rate limiting middleware to routes
5. Add username validation (alphanumeric, length 3-30)

### Week 2 - Security Hardening
1. Implement HTTP signature verification (use `ring` or `rsa` crate)
2. Add user key pair generation on creation
3. Store public keys in database (add migration)
4. Implement real RSA-SHA256 signing
5. Add HTML sanitization for RSS/Atom feeds

### Week 3 - Federation Completion
1. Fetch remote actor public keys via WebFinger
2. Complete follow request approval workflow
3. Persist incoming activities to database
4. Implement delivery retry with exponential backoff
5. Add follower/following count queries

### Ongoing
- Accessibility testing (T135-T149)
- Performance testing (T150, T157)
- Expand input validation coverage
- Security audit against ActivityPub spec

---

## Commands to Resume

```bash
# Start database
docker-compose up -d db minio minio-init

# Set up environment
cp .env.example .env
# Edit .env with real values

# Run migrations
sqlx migrate run

# Check compilation
cargo check

# Run the app
cargo run
```

---

## Files Changed in This Session

```
src/api/activitypub.rs      - ActivityPub endpoints
src/api/webfinger.rs        - WebFinger discovery
src/api/feeds.rs            - RSS/Atom feeds
src/api/oembed.rs           - oEmbed endpoint
src/api/openapi.rs          - OpenAPI documentation
src/api/middleware/rate_limit.rs - Rate limiting
src/lib/activitypub/        - ActivityPub types (actor, objects, signature)
src/jobs/                   - Background job infrastructure
migrations/20251225000012_create_indexes.sql
DEPLOYMENT.md
CONTRIBUTING.md
```

---

## Notes

- The codebase assumes external authentication (no password storage)
- ActivityPub federation is scaffolded but not functional without signature implementation
- Rate limiting layer exists but needs to be added to router in main.rs
- All sqlx queries require DATABASE_URL for compilation (use `cargo sqlx prepare` for offline)
