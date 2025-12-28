# Quickstart: Test Coverage

**Feature**: 003-test-coverage
**Date**: 2025-12-28

## Running Tests

### All Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests in specific file
cargo test --test auth_test
```

### Unit Tests Only

```bash
# Unit tests don't need database
SQLX_OFFLINE=true cargo test --lib
```

### Integration Tests Only

```bash
# Requires running database
cargo test --test '*'
```

## Prerequisites

### Database Setup

```bash
# Start PostgreSQL (using podman/docker)
make db

# Or manually:
podman run -d \
  --name oppskrift_db \
  -e POSTGRES_USER=oppskrift \
  -e POSTGRES_PASSWORD=oppskrift \
  -e POSTGRES_DB=oppskrift \
  -p 5432:5432 \
  postgres:15-alpine

# Run migrations
sqlx migrate run
```

### Environment Variables

```bash
# Required for integration tests
export DATABASE_URL="postgres://oppskrift:oppskrift@localhost:5432/oppskrift"

# Required for 2FA tests
export TOTP_ENCRYPTION_KEY="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
```

## Test Structure

### Unit Tests (inline)

```rust
// In src/services/recipe_service.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ingredients_ok() {
        let ingredients = vec![/* ... */];
        assert!(validate_ingredients(&ingredients).is_ok());
    }

    #[test]
    fn test_validate_ingredients_too_many() {
        let ingredients = (0..51).map(|_| /* ... */).collect();
        assert!(validate_ingredients(&ingredients).is_err());
    }
}
```

### Integration Tests

```rust
// In tests/recipes_test.rs
use axum_test::TestServer;
use oppskrift::create_router;

async fn setup() -> TestServer {
    let state = create_test_state().await;
    let app = create_router(state);
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn test_create_recipe() {
    let server = setup().await;
    
    // Login first
    let session = login_test_user(&server).await;
    
    // Create recipe
    let response = server
        .post("/api/recipes")
        .add_cookie(session)
        .json(&create_test_recipe())
        .await;
    
    response.assert_status_ok();
}
```

## Test Helpers

### Core Context (`tests/common/mod.rs`)

```rust
use common::TestContext;

#[tokio::test]
async fn test_example() {
    let mut ctx = TestContext::new().await;

    // Create test user directly in DB
    let user_id = ctx.create_user(
        "test@example.com",
        "testuser",
        "Password123!",
        true  // verified
    ).await;

    // Make API requests
    let response = ctx.post("/api/v1/auth/login", json!({
        "email": "test@example.com",
        "password": "Password123!"
    })).await;

    assert_eq!(response.status, 200);

    // Always cleanup at the end
    ctx.cleanup().await;
}
```

### Fixtures (`tests/common/fixtures.rs`)

```rust
use common::fixtures::*;

// Create test entities
let user_payload = create_test_user("suffix");
let recipe_payload = create_test_recipe();
let book_payload = create_test_book();

// Custom recipes
let private_recipe = create_test_recipe_with("My Recipe", "private");

// Mock ActivityPub data
let remote_actor = mock_remote_actor("alice", "remote.example");
let follow_activity = mock_follow_activity(actor_id, target_id);
```

### Assertions (`tests/common/assertions.rs`)

```rust
use common::assertions::*;

// Check JSON structure
assert_json_field(&response.body, "user_id");
assert_json_array(&response.body, "recipes");
assert_json_array_len(&response.body, "items", 10);
assert_json_uuid(&response.body, "id");

// Check status codes
assert_success_status(response.status);
assert_client_error_status(response.status);

// Check error responses
assert_error_response(&response.body);
```

### Request Methods

```rust
// Unauthenticated requests
let response = ctx.post("/api/v1/auth/register", json!({...})).await;
let response = ctx.get("/api/v1/recipes").await;

// Authenticated requests
let session = login_response.session_cookie.unwrap();
let response = ctx.get_with_session("/api/v1/users/me", &session).await;
let response = ctx.post_with_session("/api/v1/recipes", body, &session).await;
```

## Coverage Goals

| Category | Current | Target |
|----------|---------|--------|
| Services | ~50% | 90% |
| Models | ~60% | 80% |
| API | ~10% | 80% |
| Core | ~70% | 80% |
| Handlers | 0% | 50% |

## CI/CD

Tests run automatically on:
- Every push to main
- Every pull request

Configuration in `.github/workflows/ci.yml`:
- Build job compiles with SQLX_OFFLINE
- Test job runs against real PostgreSQL
