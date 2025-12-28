# Research: Test Coverage Implementation

**Feature**: 003-test-coverage
**Date**: 2025-12-28

## Research Questions

### 1. Best Practices for Rust Integration Testing with Axum

**Decision**: Use `axum-test` crate for integration testing

**Rationale**: 
- Already in use in the project (tests/auth_test.rs)
- Provides clean API for testing Axum handlers
- Supports cookies, headers, JSON bodies natively
- No need to spawn actual server (in-process testing)

**Alternatives Considered**:
- `reqwest` with real server: Slower, more complex setup
- `tower::ServiceExt`: Lower-level, more boilerplate
- `httpc-test`: Less Axum-specific

### 2. Test Database Strategy

**Decision**: Use dedicated test database with transaction rollback per test

**Rationale**:
- Current approach in auth_test.rs creates unique users per test
- SQLx supports test transactions with rollback
- PostgreSQL required for integration tests (matches production)
- SQLX_OFFLINE=true for unit tests only

**Alternatives Considered**:
- In-memory SQLite: Different behavior from PostgreSQL
- Docker per test: Too slow for 5-minute target
- Shared database with cleanup: Risk of pollution

### 3. Mocking Strategy for External Services

**Decision**: Use trait-based dependency injection with mock implementations

**Rationale**:
- Email service already uses `EmailService` struct that can be mocked
- Storage service uses configurable backends
- Password hashing already fast in tests (reduced rounds)

**Implementation Pattern**:
```rust
// Existing pattern in codebase
#[cfg(test)]
mod tests {
    use super::*;
    
    fn test_email_service() -> EmailService {
        let config = SmtpConfig::disabled();
        EmailService::new(config, "http://test.local".to_string())
    }
}
```

### 4. Test Organization Pattern

**Decision**: Follow existing Rust conventions
- Unit tests: Inline `#[cfg(test)] mod tests` blocks
- Integration tests: Separate files in `tests/` directory
- Test helpers: `tests/common/mod.rs`

**Rationale**:
- Matches existing codebase patterns
- Standard Rust convention
- Clear separation of concerns

### 5. Handler Testing Approach

**Decision**: Test handlers as integration tests, not unit tests

**Rationale**:
- Handlers depend on database state
- Template rendering requires real context
- Handler logic is primarily routing/orchestration
- Integration tests provide more value than mocked unit tests

**Exception**: Pure template rendering tests for static content

### 6. ActivityPub Federation Testing

**Decision**: Use mocked remote actors for federation tests

**Rationale**:
- Can't depend on external instances for tests
- Mock responses simulate federation scenarios
- Test both incoming and outgoing activities

**Implementation**:
- Mock HTTP responses for remote actor fetch
- Test signature verification with known keys
- Validate activity serialization/deserialization

## Test Coverage Targets

| Category | Files | Target Coverage |
|----------|-------|-----------------|
| Services | 12 | 90% (business logic) |
| Models | 17 | 80% (validation, serialization) |
| API Endpoints | 12 | 1 happy + 1 error path each |
| Core Utilities | 11 | 80% |
| Handlers | 7 | Integration tests only |
| Jobs | 2 | 80% |

## Dependencies for Testing

Already in Cargo.toml:
- `axum-test` - Integration testing
- `tokio` - Async runtime

May need to add:
- None identified - existing deps sufficient

## Test Execution Performance

**Current**: ~8 seconds for 119 tests
**Target**: <5 minutes for expanded suite

**Optimizations**:
- Parallel test execution (default in cargo test)
- Connection pooling for database tests
- Fast password hashing (already implemented)
- SQLX_OFFLINE for compile-time query checking
