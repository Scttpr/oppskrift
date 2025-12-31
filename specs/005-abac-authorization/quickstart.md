# Quickstart: ABAC Authorization System

**Feature**: 005-abac-authorization
**Date**: 2025-12-30

## Overview

This document provides a quick reference for implementing and testing the ABAC authorization system.

---

## Setup

### 1. Run Migrations

```bash
# From project root
DATABASE_URL="postgres://oppskrift:oppskrift@localhost:5432/oppskrift" sqlx migrate run
```

### 2. Prepare SQLx Offline Mode

```bash
DATABASE_URL="postgres://oppskrift:oppskrift@localhost:5432/oppskrift" cargo sqlx prepare
```

### 3. Build and Test

```bash
# Build with offline mode
SQLX_OFFLINE=true cargo build

# Run tests
DATABASE_URL="postgres://oppskrift:oppskrift@localhost:5432/oppskrift" cargo test
```

---

## Key Files to Implement

### Models (in order)

1. `src/models/visibility.rs` - Extend Visibility enum
2. `src/models/permission.rs` - PermissionLevel, SubjectType, Permission
3. `src/models/group.rs` - Group, GroupMember
4. `src/models/book_contribution.rs` - BookContribution

### Services (in order)

1. `src/services/permission_service.rs` - Core permission check logic
2. `src/services/group_service.rs` - Group management
3. Update `src/services/recipe_service.rs` - Add permission checks
4. Update `src/services/book_service.rs` - Add permission checks + contribution logic

### API (in order)

1. `src/api/permissions.rs` - Permission management endpoints
2. `src/api/groups.rs` - Group management endpoints
3. Update `src/api/recipes.rs` - Use permission service
4. Update `src/api/books.rs` - Use permission service + contributions

---

## Permission Check Flow

```rust
// In any handler or service that needs authorization:
use crate::services::PermissionService;

// Check if user can view
if !permission_service.check_permission(
    user_id,
    ResourceType::Recipe,
    recipe_id,
    PermissionLevel::View,
).await? {
    return Err(AppError::NotFound("Recipe not found".to_string()));
}

// Check if user can edit
permission_service.require_permission(
    user_id,
    ResourceType::Recipe,
    recipe_id,
    PermissionLevel::Edit,
).await?;
```

---

## Testing Patterns

### Test Permission Grants

```rust
#[tokio::test]
async fn test_share_recipe_with_user() {
    let ctx = TestContext::new().await;

    // Create owner and recipe
    let (owner_id, owner_session) = ctx.create_and_login("owner").await;
    let recipe = ctx.create_recipe(owner_id, Visibility::Private).await;

    // Create recipient
    let (recipient_id, recipient_session) = ctx.create_and_login("recipient").await;

    // Verify recipient cannot access before share
    let resp = ctx.get_recipe(recipe.id, Some(&recipient_session)).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Grant permission
    ctx.grant_permission(
        &owner_session,
        ResourceType::Recipe,
        recipe.id,
        SubjectType::User,
        Some(recipient_id),
        None,
        PermissionLevel::View,
    ).await;

    // Verify recipient can now access
    let resp = ctx.get_recipe(recipe.id, Some(&recipient_session)).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
```

### Test Group Permissions

```rust
#[tokio::test]
async fn test_group_permission_cascade() {
    let ctx = TestContext::new().await;

    // Create owner and group
    let (owner_id, owner_session) = ctx.create_and_login("owner").await;
    let group = ctx.create_group(&owner_session, "Family").await;

    // Create member and add to group
    let (member_id, member_session) = ctx.create_and_login("member").await;
    ctx.add_group_member(&owner_session, group.id, member_id).await;

    // Create recipe and share with group
    let recipe = ctx.create_recipe(owner_id, Visibility::Private).await;
    ctx.grant_permission(
        &owner_session,
        ResourceType::Recipe,
        recipe.id,
        SubjectType::Group,
        Some(group.id),
        None,
        PermissionLevel::View,
    ).await;

    // Verify member can access via group
    let resp = ctx.get_recipe(recipe.id, Some(&member_session)).await;
    assert_eq!(resp.status(), StatusCode::OK);
}
```

### Test Book Contributions

```rust
#[tokio::test]
async fn test_book_contribution() {
    let ctx = TestContext::new().await;

    // Create book owner and book
    let (owner_id, owner_session) = ctx.create_and_login("bookowner").await;
    let book = ctx.create_book(owner_id, Visibility::Private).await;

    // Create contributor
    let (contrib_id, contrib_session) = ctx.create_and_login("contributor").await;
    let recipe = ctx.create_recipe(contrib_id, Visibility::Private).await;

    // Grant contributor permission
    ctx.grant_permission(
        &owner_session,
        ResourceType::Book,
        book.id,
        SubjectType::User,
        Some(contrib_id),
        None,
        PermissionLevel::Contributor,
    ).await;

    // Contributor adds their recipe
    ctx.add_contribution(&contrib_session, book.id, recipe.id).await;

    // Verify recipe appears in book
    let book_recipes = ctx.get_book_recipes(book.id, Some(&owner_session)).await;
    assert!(book_recipes.iter().any(|r| r.id == recipe.id));

    // Verify contributor still owns the recipe
    let recipe_detail = ctx.get_recipe(recipe.id, Some(&contrib_session)).await;
    assert_eq!(recipe_detail.author_id, contrib_id);
}
```

---

## Common Queries

### Check if user has permission

```sql
-- Direct check (user permission)
SELECT EXISTS(
    SELECT 1 FROM permissions
    WHERE resource_type = 'recipe'
    AND resource_id = $1
    AND subject_type = 'user'
    AND subject_id = $2
    AND permission_level = $3
) as has_permission;
```

### Check via group membership (materialized view)

```sql
SELECT EXISTS(
    SELECT 1 FROM user_group_permissions
    WHERE user_id = $1
    AND resource_type = 'recipe'
    AND resource_id = $2
    AND permission_level = $3
) as has_permission;
```

### List resources user can access

```sql
-- Get all recipes user can view
SELECT DISTINCT r.* FROM recipes r
LEFT JOIN permissions p ON p.resource_type = 'recipe' AND p.resource_id = r.id
LEFT JOIN user_group_permissions ugp ON ugp.resource_type = 'recipe' AND ugp.resource_id = r.id
WHERE
    r.author_id = $1  -- Owner
    OR r.visibility = 'public'  -- Public
    OR (p.subject_type = 'user' AND p.subject_id = $1)  -- Direct share
    OR ugp.user_id = $1  -- Group share
ORDER BY r.created_at DESC;
```

---

## API Examples

### Grant Permission

```bash
# Share recipe with user
curl -X POST http://localhost:3000/api/v1/recipes/{recipe_id}/permissions \
  -H "Content-Type: application/json" \
  -H "Cookie: oppskrift_session=..." \
  -d '{
    "subject_type": "user",
    "subject_id": "uuid-of-recipient",
    "permission_level": "view"
  }'
```

### Create Group

```bash
curl -X POST http://localhost:3000/api/v1/groups \
  -H "Content-Type: application/json" \
  -H "Cookie: oppskrift_session=..." \
  -d '{
    "name": "Family Recipes",
    "description": "Recipes shared with family"
  }'
```

### Add Contribution

```bash
curl -X POST http://localhost:3000/api/v1/books/{book_id}/contributions \
  -H "Content-Type: application/json" \
  -H "Cookie: oppskrift_session=..." \
  -d '{
    "recipe_id": "uuid-of-my-recipe"
  }'
```

---

## Validation Rules Quick Reference

| Entity | Field | Rule |
|--------|-------|------|
| Group | name | 1-100 characters |
| Group | description | 0-500 characters |
| Permission | subject_id | Required for user/group |
| Permission | subject_domain | Required for instance |
| Permission | contributor | Only valid for books |
| BookContribution | recipe | Must be owned by contributor |
