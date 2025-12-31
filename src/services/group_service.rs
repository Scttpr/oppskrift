//! Group Service for managing groups and memberships
//!
//! Provides CRUD operations for groups and member management.

use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::models::{
    AddMemberRequest, CreateGroupRequest, Group, GroupDetail, GroupFilter, GroupMemberInfo,
    GroupWithMeta, UpdateGroupRequest,
};

/// Service for group-related operations
pub struct GroupService;

impl GroupService {
    // =========================================================================
    // Group CRUD (T054)
    // =========================================================================

    /// Create a new group
    pub async fn create(
        pool: &PgPool,
        owner_id: Uuid,
        request: CreateGroupRequest,
    ) -> AppResult<Group> {
        let id = Uuid::new_v4();

        let group = sqlx::query_as!(
            Group,
            r#"
            INSERT INTO groups (id, owner_id, name, description)
            VALUES ($1, $2, $3, $4)
            RETURNING id, owner_id, name, description, created_at, updated_at
            "#,
            id,
            owner_id,
            request.name,
            request.description
        )
        .fetch_one(pool)
        .await?;

        // Automatically add owner as first member
        sqlx::query!(
            "INSERT INTO group_members (group_id, user_id, added_by) VALUES ($1, $2, $3)",
            id,
            owner_id,
            owner_id
        )
        .execute(pool)
        .await?;

        Ok(group)
    }

    /// Get a group by ID
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> AppResult<Group> {
        let group = sqlx::query_as!(
            Group,
            "SELECT id, owner_id, name, description, created_at, updated_at FROM groups WHERE id = $1",
            id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

        Ok(group)
    }

    /// Get group with metadata (for authenticated user)
    pub async fn get_with_meta(
        pool: &PgPool,
        id: Uuid,
        viewer_id: Uuid,
    ) -> AppResult<GroupWithMeta> {
        let group = sqlx::query!(
            r#"
            SELECT
                g.id, g.owner_id, g.name, g.description, g.created_at, g.updated_at,
                COUNT(gm.user_id) as "member_count!"
            FROM groups g
            LEFT JOIN group_members gm ON gm.group_id = g.id
            WHERE g.id = $1
            GROUP BY g.id
            "#,
            id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

        Ok(GroupWithMeta {
            id: group.id,
            owner_id: group.owner_id,
            name: group.name,
            description: group.description,
            member_count: group.member_count,
            created_at: group.created_at,
            updated_at: group.updated_at,
            is_owner: group.owner_id == viewer_id,
        })
    }

    /// Get group detail with members list
    pub async fn get_detail(pool: &PgPool, id: Uuid, viewer_id: Uuid) -> AppResult<GroupDetail> {
        let group = Self::get_with_meta(pool, id, viewer_id).await?;
        let members = Self::get_members(pool, id).await?;

        Ok(GroupDetail { group, members })
    }

    /// Update a group (owner only)
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        owner_id: Uuid,
        request: UpdateGroupRequest,
    ) -> AppResult<Group> {
        // Verify ownership
        let group = Self::get_by_id(pool, id).await?;
        if group.owner_id != owner_id {
            return Err(AppError::NotFound("Group not found".to_string()));
        }

        let updated = sqlx::query_as!(
            Group,
            r#"
            UPDATE groups SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, owner_id, name, description, created_at, updated_at
            "#,
            id,
            request.name,
            request.description
        )
        .fetch_one(pool)
        .await?;

        Ok(updated)
    }

    /// Delete a group (owner only)
    pub async fn delete(pool: &PgPool, id: Uuid, owner_id: Uuid) -> AppResult<()> {
        // Verify ownership
        let group = Self::get_by_id(pool, id).await?;
        if group.owner_id != owner_id {
            return Err(AppError::NotFound("Group not found".to_string()));
        }

        // Delete group (cascades to group_members and permissions via FK)
        sqlx::query!("DELETE FROM groups WHERE id = $1", id)
            .execute(pool)
            .await?;

        Ok(())
    }

    // =========================================================================
    // Group Listing (T057)
    // =========================================================================

    /// List groups for a user (owned, member of, or all)
    pub async fn list_for_user(
        pool: &PgPool,
        user_id: Uuid,
        filter: GroupFilter,
        page: i64,
        page_size: i64,
    ) -> AppResult<(Vec<GroupWithMeta>, i64)> {
        let offset = (page - 1) * page_size;

        match filter {
            GroupFilter::All => {
                let groups = sqlx::query!(
                    r#"
                    SELECT
                        g.id, g.owner_id, g.name, g.description, g.created_at, g.updated_at,
                        COUNT(gm2.user_id) as "member_count!"
                    FROM groups g
                    JOIN group_members gm ON gm.group_id = g.id AND gm.user_id = $1
                    LEFT JOIN group_members gm2 ON gm2.group_id = g.id
                    GROUP BY g.id
                    ORDER BY g.name
                    LIMIT $2 OFFSET $3
                    "#,
                    user_id,
                    page_size,
                    offset
                )
                .fetch_all(pool)
                .await?;

                let total: i64 = sqlx::query_scalar!(
                    "SELECT COUNT(DISTINCT g.id) FROM groups g JOIN group_members gm ON gm.group_id = g.id WHERE gm.user_id = $1",
                    user_id
                )
                .fetch_one(pool)
                .await?
                .unwrap_or(0);

                let groups_with_meta: Vec<GroupWithMeta> = groups
                    .into_iter()
                    .map(|g| GroupWithMeta {
                        id: g.id,
                        owner_id: g.owner_id,
                        name: g.name,
                        description: g.description,
                        member_count: g.member_count,
                        created_at: g.created_at,
                        updated_at: g.updated_at,
                        is_owner: g.owner_id == user_id,
                    })
                    .collect();

                Ok((groups_with_meta, total))
            }
            GroupFilter::Owned => {
                let groups = sqlx::query!(
                    r#"
                    SELECT
                        g.id, g.owner_id, g.name, g.description, g.created_at, g.updated_at,
                        COUNT(gm.user_id) as "member_count!"
                    FROM groups g
                    LEFT JOIN group_members gm ON gm.group_id = g.id
                    WHERE g.owner_id = $1
                    GROUP BY g.id
                    ORDER BY g.name
                    LIMIT $2 OFFSET $3
                    "#,
                    user_id,
                    page_size,
                    offset
                )
                .fetch_all(pool)
                .await?;

                let total: i64 =
                    sqlx::query_scalar!("SELECT COUNT(*) FROM groups WHERE owner_id = $1", user_id)
                        .fetch_one(pool)
                        .await?
                        .unwrap_or(0);

                let groups_with_meta: Vec<GroupWithMeta> = groups
                    .into_iter()
                    .map(|g| GroupWithMeta {
                        id: g.id,
                        owner_id: g.owner_id,
                        name: g.name,
                        description: g.description,
                        member_count: g.member_count,
                        created_at: g.created_at,
                        updated_at: g.updated_at,
                        is_owner: true, // Always true for Owned filter
                    })
                    .collect();

                Ok((groups_with_meta, total))
            }
            GroupFilter::Member => {
                let groups = sqlx::query!(
                    r#"
                    SELECT
                        g.id, g.owner_id, g.name, g.description, g.created_at, g.updated_at,
                        COUNT(gm2.user_id) as "member_count!"
                    FROM groups g
                    JOIN group_members gm ON gm.group_id = g.id AND gm.user_id = $1
                    LEFT JOIN group_members gm2 ON gm2.group_id = g.id
                    WHERE g.owner_id != $1
                    GROUP BY g.id
                    ORDER BY g.name
                    LIMIT $2 OFFSET $3
                    "#,
                    user_id,
                    page_size,
                    offset
                )
                .fetch_all(pool)
                .await?;

                let total: i64 = sqlx::query_scalar!(
                    "SELECT COUNT(DISTINCT g.id) FROM groups g JOIN group_members gm ON gm.group_id = g.id WHERE gm.user_id = $1 AND g.owner_id != $1",
                    user_id
                )
                .fetch_one(pool)
                .await?
                .unwrap_or(0);

                let groups_with_meta: Vec<GroupWithMeta> = groups
                    .into_iter()
                    .map(|g| GroupWithMeta {
                        id: g.id,
                        owner_id: g.owner_id,
                        name: g.name,
                        description: g.description,
                        member_count: g.member_count,
                        created_at: g.created_at,
                        updated_at: g.updated_at,
                        is_owner: false, // Always false for Member filter
                    })
                    .collect();

                Ok((groups_with_meta, total))
            }
        }
    }

    // =========================================================================
    // Member Management (T055-T056)
    // =========================================================================

    /// Add a member to a group (owner only)
    pub async fn add_member(
        pool: &PgPool,
        group_id: Uuid,
        owner_id: Uuid,
        request: AddMemberRequest,
    ) -> AppResult<()> {
        // Verify ownership
        let group = Self::get_by_id(pool, group_id).await?;
        if group.owner_id != owner_id {
            return Err(AppError::NotFound("Group not found".to_string()));
        }

        // Check if user exists
        let user_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM users WHERE id = $1) as \"exists!\"",
            request.user_id
        )
        .fetch_one(pool)
        .await?;

        if !user_exists {
            return Err(AppError::NotFound("User not found".to_string()));
        }

        // Add member (ignore if already exists)
        sqlx::query!(
            r#"
            INSERT INTO group_members (group_id, user_id, added_by)
            VALUES ($1, $2, $3)
            ON CONFLICT (group_id, user_id) DO NOTHING
            "#,
            group_id,
            request.user_id,
            owner_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Remove a member from a group (owner only, cannot remove self if owner)
    pub async fn remove_member(
        pool: &PgPool,
        group_id: Uuid,
        owner_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<()> {
        // Verify ownership
        let group = Self::get_by_id(pool, group_id).await?;
        if group.owner_id != owner_id {
            return Err(AppError::NotFound("Group not found".to_string()));
        }

        // Cannot remove the owner
        if user_id == owner_id {
            return Err(AppError::BadRequest(
                "Cannot remove owner from group. Transfer ownership or delete the group."
                    .to_string(),
            ));
        }

        sqlx::query!(
            "DELETE FROM group_members WHERE group_id = $1 AND user_id = $2",
            group_id,
            user_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Leave a group (any member, owner cannot leave)
    pub async fn leave(pool: &PgPool, group_id: Uuid, user_id: Uuid) -> AppResult<()> {
        let group = Self::get_by_id(pool, group_id).await?;

        // Owner cannot leave their own group
        if group.owner_id == user_id {
            return Err(AppError::BadRequest(
                "Owner cannot leave group. Transfer ownership or delete the group.".to_string(),
            ));
        }

        // Check if user is a member
        let is_member = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2) as \"exists!\"",
            group_id,
            user_id
        )
        .fetch_one(pool)
        .await?;

        if !is_member {
            return Err(AppError::NotFound("Not a member of this group".to_string()));
        }

        sqlx::query!(
            "DELETE FROM group_members WHERE group_id = $1 AND user_id = $2",
            group_id,
            user_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get members of a group
    pub async fn get_members(pool: &PgPool, group_id: Uuid) -> AppResult<Vec<GroupMemberInfo>> {
        let members = sqlx::query!(
            r#"
            SELECT
                u.id as user_id,
                u.username,
                u.display_name,
                u.avatar_url,
                gm.added_at,
                gm.added_by
            FROM group_members gm
            JOIN users u ON u.id = gm.user_id
            WHERE gm.group_id = $1
            ORDER BY gm.added_at
            "#,
            group_id
        )
        .fetch_all(pool)
        .await?;

        Ok(members
            .into_iter()
            .map(|m| GroupMemberInfo {
                user_id: m.user_id,
                username: m.username,
                display_name: m.display_name,
                avatar_url: m.avatar_url,
                added_at: m.added_at,
                added_by: m.added_by,
            })
            .collect())
    }

    /// Check if user is a member of a group
    pub async fn is_member(pool: &PgPool, group_id: Uuid, user_id: Uuid) -> AppResult<bool> {
        let is_member = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM group_members WHERE group_id = $1 AND user_id = $2) as \"exists!\"",
            group_id,
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(is_member)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_filter_default() {
        assert_eq!(GroupFilter::default(), GroupFilter::All);
    }
}
