//! Permission Service for ABAC authorization
//!
//! Implements the core permission checking logic with the following evaluation order:
//! 1. Owner check (short-circuit)
//! 2. Direct user permission
//! 3. Group membership permission
//! 4. Followers check (for followers_only visibility)
//! 5. Public visibility check
//!
//! When multiple permission paths exist, the highest permission level wins.
//!
//! ## Performance Notes
//!
//! For high-traffic deployments, permission checks can be cached with a short TTL
//! (e.g., 30 seconds) using a cache like `moka`. Cache should be invalidated on:
//! - Permission grant/revoke
//! - User/group deletion
//! - Resource visibility changes
//!
//! Current implementation relies on database query efficiency and connection pooling.

use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::models::{
    CreateAuditLog, GrantPermissionRequest, Permission, PermissionLevel, PermissionWithDisplay,
    ResourceType, SubjectType, Visibility,
};

/// Service for permission-related operations
pub struct PermissionService;

/// Result of a permission check with the effective level
#[derive(Debug, Clone)]
pub struct PermissionCheckResult {
    pub has_permission: bool,
    pub effective_level: Option<PermissionLevel>,
    pub reason: PermissionReason,
}

/// Reason for the permission decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionReason {
    /// User is the resource owner
    Owner,
    /// User has direct permission grant
    DirectPermission,
    /// User has permission via group membership
    GroupPermission,
    /// User is a follower and resource is followers_only
    Follower,
    /// Resource is public
    PublicVisibility,
    /// User has permission via instance share
    InstancePermission,
    /// No permission found
    NoPermission,
}

impl PermissionService {
    // =========================================================================
    // Core Permission Checking (T017-T019)
    // =========================================================================

    /// Check if a user has the required permission level on a resource
    ///
    /// Evaluation order (short-circuits on success):
    /// 1. Owner → full access
    /// 2. Direct user permission
    /// 3. Group membership permission
    /// 4. Instance permission (federated)
    /// 5. Followers (for followers_only visibility)
    /// 6. Public visibility
    pub async fn check_permission(
        pool: &PgPool,
        user_id: Option<Uuid>,
        resource_type: ResourceType,
        resource_id: Uuid,
        required_level: PermissionLevel,
    ) -> AppResult<PermissionCheckResult> {
        // First, get the resource owner and visibility
        let (owner_id, visibility) =
            Self::get_resource_info(pool, resource_type, resource_id).await?;

        // 1. Owner check - owners have full access
        if let Some(uid) = user_id {
            if uid == owner_id {
                return Ok(PermissionCheckResult {
                    has_permission: true,
                    effective_level: Some(PermissionLevel::Edit),
                    reason: PermissionReason::Owner,
                });
            }
        }

        // Collect all permission levels from various sources
        let mut highest_level: Option<PermissionLevel> = None;
        let mut reason = PermissionReason::NoPermission;

        // 2. Check direct user permission
        if let Some(uid) = user_id {
            if let Some(level) =
                Self::get_direct_permission(pool, resource_type, resource_id, uid).await?
            {
                if highest_level.map_or(true, |h| level.rank() > h.rank()) {
                    highest_level = Some(level);
                    reason = PermissionReason::DirectPermission;
                }
            }
        }

        // 3. Check group permissions
        if let Some(uid) = user_id {
            if let Some(level) =
                Self::get_group_permission(pool, resource_type, resource_id, uid).await?
            {
                if highest_level.map_or(true, |h| level.rank() > h.rank()) {
                    highest_level = Some(level);
                    reason = PermissionReason::GroupPermission;
                }
            }
        }

        // 4. Check instance permission (for federated users)
        if let Some(uid) = user_id {
            if let Some(level) =
                Self::get_instance_permission(pool, resource_type, resource_id, uid).await?
            {
                if highest_level.map_or(true, |h| level.rank() > h.rank()) {
                    highest_level = Some(level);
                    reason = PermissionReason::InstancePermission;
                }
            }
        }

        // 5. Check followers_only visibility
        if visibility == Visibility::FollowersOnly {
            if let Some(uid) = user_id {
                if Self::is_follower(pool, uid, owner_id).await? {
                    // Followers get view access
                    if highest_level.map_or(true, |h| PermissionLevel::View.rank() > h.rank()) {
                        highest_level = Some(PermissionLevel::View);
                        reason = PermissionReason::Follower;
                    }
                }
            }
        }

        // 6. Check public visibility
        if visibility == Visibility::Public {
            // Public grants view access
            if highest_level.map_or(true, |h| PermissionLevel::View.rank() > h.rank()) {
                highest_level = Some(PermissionLevel::View);
                reason = PermissionReason::PublicVisibility;
            }
        }

        // Determine if the effective level meets the required level
        let has_permission = highest_level.is_some_and(|level| level.grants(required_level));

        Ok(PermissionCheckResult {
            has_permission,
            effective_level: highest_level,
            reason,
        })
    }

    /// Require a permission level, returning NotFound error if not granted
    /// (Per spec: unauthorized access returns 404, not 403)
    pub async fn require_permission(
        pool: &PgPool,
        user_id: Option<Uuid>,
        resource_type: ResourceType,
        resource_id: Uuid,
        required_level: PermissionLevel,
    ) -> AppResult<PermissionCheckResult> {
        let result =
            Self::check_permission(pool, user_id, resource_type, resource_id, required_level)
                .await?;

        if !result.has_permission {
            // Log access denied
            Self::log_audit(
                pool,
                CreateAuditLog::access_denied(
                    user_id,
                    resource_type.as_str(),
                    resource_id,
                    &required_level.to_string().to_lowercase(),
                ),
            )
            .await?;

            // Return 404 to hide resource existence
            let resource_name = match resource_type {
                ResourceType::Recipe => "Recipe",
                ResourceType::Book => "Book",
            };
            return Err(AppError::NotFound(format!("{} not found", resource_name)));
        }

        Ok(result)
    }

    /// Check if user is the owner of a resource
    pub async fn is_owner(
        pool: &PgPool,
        user_id: Uuid,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> AppResult<bool> {
        let (owner_id, _) = Self::get_resource_info(pool, resource_type, resource_id).await?;
        Ok(user_id == owner_id)
    }

    // =========================================================================
    // Permission Grant/Revoke Operations (T036-T039)
    // =========================================================================

    /// Grant a permission to a subject
    pub async fn grant_permission(
        pool: &PgPool,
        granter_id: Uuid,
        resource_type: ResourceType,
        resource_id: Uuid,
        request: GrantPermissionRequest,
    ) -> AppResult<Permission> {
        // Validate the request
        request
            .validate()
            .map_err(|e| AppError::BadRequest(e.to_string()))?;

        // Verify granter is the owner
        if !Self::is_owner(pool, granter_id, resource_type, resource_id).await? {
            return Err(AppError::NotFound(format!(
                "{} not found",
                match resource_type {
                    ResourceType::Recipe => "Recipe",
                    ResourceType::Book => "Book",
                }
            )));
        }

        // Contributor level only valid for books
        if request.permission_level == PermissionLevel::Contributor
            && resource_type != ResourceType::Book
        {
            return Err(AppError::BadRequest(
                "Contributor permission level is only valid for books".to_string(),
            ));
        }

        let id = Uuid::new_v4();
        let resource_type_str = resource_type.as_str();

        let permission = sqlx::query_as!(
            Permission,
            r#"
            INSERT INTO permissions (
                id, resource_type, resource_id, subject_type, subject_id,
                subject_domain, permission_level, granted_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id, resource_type, resource_id,
                subject_type as "subject_type: SubjectType",
                subject_id, subject_domain,
                permission_level as "permission_level: PermissionLevel",
                granted_by, granted_at
            "#,
            id,
            resource_type_str,
            resource_id,
            request.subject_type as SubjectType,
            request.subject_id,
            request.subject_domain,
            request.permission_level as PermissionLevel,
            granter_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err)
                if db_err.constraint().is_some()
                    && db_err.constraint().unwrap().contains("unique") =>
            {
                AppError::Conflict("Permission already exists".to_string())
            }
            _ => AppError::from(e),
        })?;

        // Audit log
        Self::log_audit(
            pool,
            CreateAuditLog::permission_granted(
                granter_id,
                resource_type_str,
                resource_id,
                &request.subject_type.to_string().to_lowercase(),
                request.subject_id,
                &request.permission_level.to_string().to_lowercase(),
            ),
        )
        .await?;

        Ok(permission)
    }

    /// Revoke a permission
    pub async fn revoke_permission(
        pool: &PgPool,
        revoker_id: Uuid,
        resource_type: ResourceType,
        resource_id: Uuid,
        permission_id: Uuid,
    ) -> AppResult<()> {
        // Verify revoker is the owner
        if !Self::is_owner(pool, revoker_id, resource_type, resource_id).await? {
            return Err(AppError::NotFound(format!(
                "{} not found",
                match resource_type {
                    ResourceType::Recipe => "Recipe",
                    ResourceType::Book => "Book",
                }
            )));
        }

        // Get permission details for audit log before deleting
        let permission = sqlx::query_as!(
            Permission,
            r#"
            SELECT
                id, resource_type, resource_id,
                subject_type as "subject_type: SubjectType",
                subject_id, subject_domain,
                permission_level as "permission_level: PermissionLevel",
                granted_by, granted_at
            FROM permissions
            WHERE id = $1 AND resource_type = $2 AND resource_id = $3
            "#,
            permission_id,
            resource_type.as_str(),
            resource_id
        )
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Permission not found".to_string()))?;

        // Delete the permission
        sqlx::query!("DELETE FROM permissions WHERE id = $1", permission_id)
            .execute(pool)
            .await?;

        // Audit log
        Self::log_audit(
            pool,
            CreateAuditLog::permission_revoked(
                revoker_id,
                &permission.resource_type,
                permission.resource_id,
                &permission.subject_type.to_string().to_lowercase(),
                permission.subject_id,
                &permission.permission_level.to_string().to_lowercase(),
            ),
        )
        .await?;

        Ok(())
    }

    /// List all permissions for a resource
    pub async fn list_permissions(
        pool: &PgPool,
        user_id: Uuid,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> AppResult<Vec<PermissionWithDisplay>> {
        // Verify user is the owner
        if !Self::is_owner(pool, user_id, resource_type, resource_id).await? {
            return Err(AppError::NotFound(format!(
                "{} not found",
                match resource_type {
                    ResourceType::Recipe => "Recipe",
                    ResourceType::Book => "Book",
                }
            )));
        }

        let permissions = sqlx::query!(
            r#"
            SELECT
                p.id, p.resource_type, p.resource_id,
                p.subject_type as "subject_type: SubjectType",
                p.subject_id, p.subject_domain,
                p.permission_level as "permission_level: PermissionLevel",
                p.granted_by, p.granted_at,
                COALESCE(
                    u.display_name,
                    g.name,
                    p.subject_domain,
                    'Unknown'
                ) as subject_display_name
            FROM permissions p
            LEFT JOIN users u ON p.subject_type = 'user' AND p.subject_id = u.id
            LEFT JOIN groups g ON p.subject_type = 'group' AND p.subject_id = g.id
            WHERE p.resource_type = $1 AND p.resource_id = $2
            ORDER BY p.granted_at DESC
            "#,
            resource_type.as_str(),
            resource_id
        )
        .fetch_all(pool)
        .await?;

        Ok(permissions
            .into_iter()
            .map(|p| PermissionWithDisplay {
                id: p.id,
                resource_type: p.resource_type,
                resource_id: p.resource_id,
                subject_type: p.subject_type,
                subject_id: p.subject_id,
                subject_domain: p.subject_domain,
                subject_display_name: p
                    .subject_display_name
                    .unwrap_or_else(|| "Unknown".to_string()),
                permission_level: p.permission_level,
                granted_by: p.granted_by,
                granted_at: p.granted_at,
            })
            .collect())
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Get resource owner ID and visibility
    async fn get_resource_info(
        pool: &PgPool,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> AppResult<(Uuid, Visibility)> {
        match resource_type {
            ResourceType::Recipe => {
                let result = sqlx::query!(
                    r#"
                    SELECT author_id, visibility as "visibility: Visibility"
                    FROM recipes WHERE id = $1
                    "#,
                    resource_id
                )
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::NotFound("Recipe not found".to_string()))?;

                Ok((result.author_id, result.visibility))
            }
            ResourceType::Book => {
                let result = sqlx::query!(
                    r#"
                    SELECT owner_id, visibility as "visibility: Visibility"
                    FROM recipe_books WHERE id = $1
                    "#,
                    resource_id
                )
                .fetch_optional(pool)
                .await?
                .ok_or_else(|| AppError::NotFound("Book not found".to_string()))?;

                Ok((result.owner_id, result.visibility))
            }
        }
    }

    /// Get direct user permission level
    async fn get_direct_permission(
        pool: &PgPool,
        resource_type: ResourceType,
        resource_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Option<PermissionLevel>> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT permission_level as "permission_level: PermissionLevel"
            FROM permissions
            WHERE resource_type = $1
              AND resource_id = $2
              AND subject_type = 'user'
              AND subject_id = $3
            "#,
            resource_type.as_str(),
            resource_id,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(result)
    }

    /// Get highest permission level from group memberships
    async fn get_group_permission(
        pool: &PgPool,
        resource_type: ResourceType,
        resource_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Option<PermissionLevel>> {
        // Get all group permissions for groups the user is a member of
        let result = sqlx::query_scalar!(
            r#"
            SELECT p.permission_level as "permission_level: PermissionLevel"
            FROM permissions p
            JOIN group_members gm ON p.subject_id = gm.group_id
            WHERE p.resource_type = $1
              AND p.resource_id = $2
              AND p.subject_type = 'group'
              AND gm.user_id = $3
            ORDER BY
                CASE p.permission_level
                    WHEN 'edit' THEN 3
                    WHEN 'contributor' THEN 2
                    WHEN 'view' THEN 1
                END DESC
            LIMIT 1
            "#,
            resource_type.as_str(),
            resource_id,
            user_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(result)
    }

    /// Get instance permission for federated user
    async fn get_instance_permission(
        pool: &PgPool,
        resource_type: ResourceType,
        resource_id: Uuid,
        user_id: Uuid,
    ) -> AppResult<Option<PermissionLevel>> {
        // Get user's ap_id to extract instance domain
        let user_domain = sqlx::query_scalar!(
            r#"
            SELECT
                CASE
                    WHEN ap_id IS NOT NULL AND ap_id != ''
                    THEN substring(ap_id from '://([^/]+)')
                    ELSE NULL
                END as domain
            FROM users WHERE id = $1
            "#,
            user_id
        )
        .fetch_optional(pool)
        .await?
        .flatten();

        let Some(domain) = user_domain else {
            return Ok(None);
        };

        let result = sqlx::query_scalar!(
            r#"
            SELECT permission_level as "permission_level: PermissionLevel"
            FROM permissions
            WHERE resource_type = $1
              AND resource_id = $2
              AND subject_type = 'instance'
              AND subject_domain = $3
            "#,
            resource_type.as_str(),
            resource_id,
            domain
        )
        .fetch_optional(pool)
        .await?;

        Ok(result)
    }

    /// Check if user follows the resource owner
    async fn is_follower(pool: &PgPool, follower_id: Uuid, owner_id: Uuid) -> AppResult<bool> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM follows
                WHERE follower_id = $1 AND following_id = $2
            ) as "exists!"
            "#,
            follower_id,
            owner_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    // =========================================================================
    // Audit Logging (T020)
    // =========================================================================

    /// Log an audit event
    pub async fn log_audit(pool: &PgPool, log: CreateAuditLog) -> AppResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO permission_audit_log (
                event_type, actor_id, resource_type, resource_id,
                subject_type, subject_id, permission_level, details
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            log.event_type.as_str(),
            log.actor_id,
            log.resource_type,
            log.resource_id,
            log.subject_type,
            log.subject_id,
            log.permission_level,
            log.details
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    // =========================================================================
    // Privilege Escalation Prevention (T098)
    // =========================================================================

    /// Verify granter can grant the requested permission level
    ///
    /// Prevents privilege escalation by ensuring no one can grant a permission
    /// level higher than their own effective level on the resource.
    /// Owners implicitly have Edit level (highest).
    pub async fn can_grant_level(
        pool: &PgPool,
        granter_id: Uuid,
        resource_type: ResourceType,
        resource_id: Uuid,
        requested_level: PermissionLevel,
    ) -> AppResult<bool> {
        // Get granter's effective permission
        let result = Self::check_permission(
            pool,
            Some(granter_id),
            resource_type,
            resource_id,
            PermissionLevel::View,
        )
        .await?;

        // Owner check - owners can grant any level
        if result.reason == PermissionReason::Owner {
            return Ok(true);
        }

        // Non-owners can only grant up to their own level
        match result.effective_level {
            Some(level) => Ok(level.grants(requested_level)),
            None => Ok(false),
        }
    }

    // =========================================================================
    // Orphaned Permission Cleanup (T099)
    // =========================================================================

    /// Clean up permissions when a user is deleted
    pub async fn cleanup_user_permissions(pool: &PgPool, user_id: Uuid) -> AppResult<u64> {
        let result = sqlx::query!(
            "DELETE FROM permissions WHERE subject_type = 'user' AND subject_id = $1",
            user_id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Clean up permissions when a group is deleted
    pub async fn cleanup_group_permissions(pool: &PgPool, group_id: Uuid) -> AppResult<u64> {
        let result = sqlx::query!(
            "DELETE FROM permissions WHERE subject_type = 'group' AND subject_id = $1",
            group_id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Clean up permissions when a resource is deleted
    pub async fn cleanup_resource_permissions(
        pool: &PgPool,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> AppResult<u64> {
        let result = sqlx::query!(
            "DELETE FROM permissions WHERE resource_type = $1 AND resource_id = $2",
            resource_type.as_str(),
            resource_id
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Clean up orphaned permissions (resources that no longer exist)
    /// Should be run periodically as a maintenance task
    pub async fn cleanup_orphaned_permissions(pool: &PgPool) -> AppResult<u64> {
        // Delete recipe permissions where recipe no longer exists
        let recipe_result = sqlx::query!(
            r#"
            DELETE FROM permissions p
            WHERE p.resource_type = 'recipe'
            AND NOT EXISTS (SELECT 1 FROM recipes r WHERE r.id = p.resource_id)
            "#
        )
        .execute(pool)
        .await?;

        // Delete book permissions where book no longer exists
        let book_result = sqlx::query!(
            r#"
            DELETE FROM permissions p
            WHERE p.resource_type = 'book'
            AND NOT EXISTS (SELECT 1 FROM recipe_books b WHERE b.id = p.resource_id)
            "#
        )
        .execute(pool)
        .await?;

        // Delete user permissions where user no longer exists
        let user_result = sqlx::query!(
            r#"
            DELETE FROM permissions p
            WHERE p.subject_type = 'user'
            AND NOT EXISTS (SELECT 1 FROM users u WHERE u.id = p.subject_id)
            "#
        )
        .execute(pool)
        .await?;

        // Delete group permissions where group no longer exists
        let group_result = sqlx::query!(
            r#"
            DELETE FROM permissions p
            WHERE p.subject_type = 'group'
            AND NOT EXISTS (SELECT 1 FROM groups g WHERE g.id = p.subject_id)
            "#
        )
        .execute(pool)
        .await?;

        Ok(recipe_result.rows_affected()
            + book_result.rows_affected()
            + user_result.rows_affected()
            + group_result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_reason_equality() {
        assert_eq!(PermissionReason::Owner, PermissionReason::Owner);
        assert_ne!(PermissionReason::Owner, PermissionReason::DirectPermission);
    }

    #[test]
    fn test_permission_check_result_construction() {
        let result = PermissionCheckResult {
            has_permission: true,
            effective_level: Some(PermissionLevel::Edit),
            reason: PermissionReason::Owner,
        };
        assert!(result.has_permission);
        assert_eq!(result.effective_level, Some(PermissionLevel::Edit));
        assert_eq!(result.reason, PermissionReason::Owner);
    }
}
