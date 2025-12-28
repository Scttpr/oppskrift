# Data Model: Test Coverage

**Feature**: 003-test-coverage
**Date**: 2025-12-28

## Overview

This feature does not introduce new data entities. It adds tests for existing entities and their operations.

## Entities Under Test

The following existing entities require test coverage:

### User Domain
- `User` - Registration, authentication, profile
- `Session` - Login sessions, token validation
- `SecurityEvent` - Audit logging

### Recipe Domain
- `Recipe` - CRUD operations, visibility
- `Ingredient` - Recipe components
- `InstructionStep` - Recipe steps
- `RecipeBook` - Collections
- `BookRecipeEntry` - Book membership
- `SavedRecipe` - User bookmarks

### Social Domain
- `Follow` - User following
- `Activity` - Activity feed

### Federation Domain
- `ActivityPub objects` - Remote actors, activities

## Test Fixtures

### Standard Test Users

```rust
// tests/common/mod.rs
pub fn create_test_user(suffix: &str) -> RegisterRequest {
    RegisterRequest {
        username: format!("testuser_{}", suffix),
        email: format!("test_{}@example.com", suffix),
        password: "TestPassword123!".to_string(),
    }
}
```

### Standard Test Recipes

```rust
pub fn create_test_recipe(user_id: Uuid) -> CreateRecipeRequest {
    CreateRecipeRequest {
        title: "Test Recipe".to_string(),
        description: Some("A test recipe".to_string()),
        prep_time: Some(10),
        cook_time: Some(20),
        servings: Some(4),
        difficulty: Some(Difficulty::Medium),
        visibility: Visibility::Public,
        ingredients: vec![/* ... */],
        instructions: vec![/* ... */],
    }
}
```

## Validation Rules to Test

| Entity | Rule | Test Case |
|--------|------|-----------|
| User.username | 3-30 chars, alphanumeric | Too short, too long, special chars |
| User.email | Valid email format | Invalid format, duplicate |
| User.password | 8+ chars, complexity | Too short, no uppercase, no digit |
| Recipe.title | Required, max 200 | Empty, too long |
| Recipe.ingredients | Max 50 items | Exceed limit |
| Recipe.instructions | Max 100 steps | Exceed limit |
| Follow | No self-follow | User follows themselves |

## State Transitions to Test

### User States
- Unverified → Verified (email confirmation)
- Active → Locked (failed logins)
- Active → Deletion Requested → Deleted

### Recipe States
- Draft → Published (visibility change)
- Public → Private → Public

### Session States
- Active → Expired (timeout)
- Active → Revoked (logout/security)
