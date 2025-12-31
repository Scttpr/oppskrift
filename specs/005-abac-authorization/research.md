# Research: ABAC Authorization System

**Feature**: 005-abac-authorization
**Date**: 2025-12-30

## Overview

This document captures research findings for implementing Attribute-Based Access Control in the Oppskrift Rust/Axum application.

---

## 1. Permission System Design

### Decision: Denormalized Permission Table

**Rationale**: A single `permissions` table with indexed lookups provides the best balance of query performance and flexibility. Using composite indexes on (resource_type, resource_id, principal_type, principal_id, permission) enables sub-millisecond permission checks.

**Alternatives Considered**:
- Separate tables per resource type: Rejected due to query complexity and maintenance burden
- JSONB permission arrays: Rejected due to index limitations and update complexity
- External policy engine (Cedar, OPA): Rejected as overkill for current requirements

### Decision: Hybrid Authorization (Middleware + Service Layer)

**Rationale**:
- Middleware handles authentication and basic public/owner checks (fast path)
- Service layer handles fine-grained ABAC decisions (complex path)
- This avoids duplicating logic while keeping handlers clean

**Alternatives Considered**:
- Middleware-only: Rejected because ABAC decisions often need business context
- Service-only: Rejected because it clutters every handler with permission checks

### Decision: Two-Tier Caching with Invalidation

**Rationale**: Permission checks happen on every request. Using `moka` crate for in-memory caching with:
- Hot cache: 10 seconds TTL, 10K entries (per-request locality)
- Warm cache: 5 minutes TTL, 100K entries (cross-request efficiency)
- Group cache: 10 minutes TTL for group memberships

**Alternatives Considered**:
- No caching: Rejected due to performance requirements (<50ms for 95%)
- Redis-only: Rejected for single-instance deployment; Redis optional for multi-instance
- Longer TTL: Rejected because permission changes must take effect within 1 second

---

## 2. Group Membership at Scale

### Decision: Materialized View for Group Permissions

**Rationale**: A materialized view `user_group_permissions` pre-joins group memberships with permissions, enabling O(1) lookups for "does user X have permission Y on resource Z via any group?"

**Refresh Strategy**:
- Concurrent refresh on permission or membership changes
- Triggered by application event queue (async, non-blocking)

**Alternatives Considered**:
- Join at query time: Rejected for groups with 1000+ members
- Denormalized columns: Rejected due to update complexity

### Decision: Event-Driven Propagation

**Rationale**: Permission and membership changes emit events processed by a background task. This:
- Keeps API responses fast
- Ensures cache invalidation
- Maintains materialized view freshness

**Implementation**: Tokio mpsc channel for events, background task for processing.

---

## 3. Multi-Path Permission Evaluation

### Decision: Short-Circuit Evaluation Order

**Order**: Owner → Direct Share → Group Membership → Followers → Public

**Rationale**:
1. Owner check is a single column lookup (fastest)
2. Direct grants use indexed query (fast)
3. Group grants use materialized view (medium)
4. Follower check requires follow table join (slower)
5. Public check is fallback (rare for private content)

**Optimization**: For view permission, a single UNION query can check all paths at once.

### Decision: Batch Permission Filtering

**Rationale**: List views need to filter multiple resources. Using `ANY($1::uuid[])` with a UNION query checks all permission paths for all resources in one round trip.

---

## 4. Federation Integration

### Decision: Map Permissions to ActivityPub Addressing

**Rationale**: ActivityPub uses `to` and `cc` arrays for access control. Our permission model maps as:
- Public → `to: ["https://www.w3.org/ns/activitystreams#Public"]`
- Direct user share → `to: [user_actor_url]`
- Group share → `cc: [group_collection_url]`
- Followers-only → `cc: [owner_followers_url]`

### Decision: Instance-Level Permissions via Domain Allowlist

**Rationale**: Instance sharing is implemented as domain-based policy. When sharing with an instance:
- All actors from that domain inherit the permission
- Checked by extracting domain from actor URL

**Alternatives Considered**:
- Per-user grants for all instance members: Rejected as impractical
- Instance-level AP collections: Considered for future enhancement

---

## 5. Audit Logging

### Decision: Append-Only Partitioned Table

**Rationale**: Audit logs are write-heavy and rarely updated. Using:
- Monthly partitioning for efficient archival
- Batch inserts via buffered writer
- JSONB details column for flexibility

### Events to Log:
- Permission changes: granted, revoked, expired
- Access attempts: granted, denied (for security monitoring)
- Group changes: created, deleted, membership changes
- Visibility changes: public ↔ private ↔ followers-only

### Decision: Async Non-Blocking Writes

**Rationale**: Audit logging must not slow down API responses. Events are sent to an mpsc channel and processed by a background task with batch inserts.

---

## 6. Recommended Crates

| Purpose | Crate | Version | Notes |
|---------|-------|---------|-------|
| In-memory cache | `moka` | 0.12+ | High-performance concurrent cache |
| Distributed cache | `redis` | 0.27+ | Optional, for multi-instance |
| Database | `sqlx` | 0.8 | Already in use |
| Async channels | `tokio` | 1.x | Already in use |

**No new dependencies required for core functionality.** The `moka` crate is recommended for caching but could be deferred to a performance optimization phase.

---

## 7. Key Implementation Patterns

### Permission Check Function

```rust
// Order: owner → direct → group → followers → public
pub async fn check_permission(
    user: &AuthUser,
    resource_type: ResourceType,
    resource_id: Uuid,
    permission: PermissionLevel,
) -> Result<bool, AppError>
```

### Permission Grant Request

```rust
pub struct GrantPermissionRequest {
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    pub subject_type: SubjectType,  // User, Group, Instance
    pub subject_id: Option<Uuid>,   // None for Instance (domain-based)
    pub subject_domain: Option<String>,  // For Instance
    pub permission: PermissionLevel,
}
```

### Cache Invalidation Events

```rust
pub enum PermissionEvent {
    GrantAdded { grant: Permission },
    GrantRemoved { resource_type, resource_id, subject_type, subject_id },
    GroupMembershipChanged { group_id, user_id, action },
}
```

---

## 8. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Cache inconsistency | Event-driven invalidation + short TTL on hot cache |
| Group permission lag | Concurrent materialized view refresh |
| Permission check latency | Short-circuit evaluation + caching |
| Audit log growth | Monthly partitioning + archival policy |
| Federation complexity | Start with instance blocking only; full instance sharing in later phase |

---

## 9. Phased Implementation

**Phase 1 (MVP)**: Owner + Direct Share + Public
- Visibility enum extension (FollowersOnly)
- Permission table + basic CRUD
- Permission check in services
- Basic audit logging

**Phase 2**: Groups + Followers
- Group management
- Group permissions with materialized view
- Followers-only visibility

**Phase 3**: Federation + Advanced
- Instance-level permissions
- AP addressing integration
- Permission caching with moka
- Batch permission filtering
