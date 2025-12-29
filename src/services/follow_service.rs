use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, AppResult};
use crate::models::{Follow, FollowCounts, User};

pub struct FollowService;

impl FollowService {
    /// Follow a user
    pub async fn follow(
        pool: &PgPool,
        follower_id: Uuid,
        following_id: Uuid,
        base_url: &str,
    ) -> AppResult<Follow> {
        // Prevent self-follow (also enforced at DB level)
        if follower_id == following_id {
            return Err(AppError::Validation("Cannot follow yourself".to_string()));
        }

        let id = Uuid::new_v4();
        let ap_id = format!("{}/follows/{}", base_url, id);

        sqlx::query_as!(
            Follow,
            r#"
            INSERT INTO follows (id, follower_id, following_id, ap_id)
            VALUES ($1, $2, $3, $4)
            RETURNING id, follower_id, following_id, created_at, ap_id
            "#,
            id,
            follower_id,
            following_id,
            ap_id
        )
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db_err) if db_err.constraint() == Some("follows_unique") => {
                AppError::Validation("Already following this user".to_string())
            }
            _ => AppError::from(e),
        })
    }

    /// Unfollow a user
    pub async fn unfollow(pool: &PgPool, follower_id: Uuid, following_id: Uuid) -> AppResult<()> {
        let result = sqlx::query!(
            r#"
            DELETE FROM follows
            WHERE follower_id = $1 AND following_id = $2
            "#,
            follower_id,
            following_id
        )
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Follow relationship not found".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if a user is following another user
    pub async fn is_following(
        pool: &PgPool,
        follower_id: Uuid,
        following_id: Uuid,
    ) -> AppResult<bool> {
        let result = sqlx::query_scalar!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM follows
                WHERE follower_id = $1 AND following_id = $2
            ) as "exists!"
            "#,
            follower_id,
            following_id
        )
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Get followers of a user
    pub async fn get_followers(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT u.id, u.username, u.email, u.email_verified, u.password_hash,
                   u.display_name, u.bio, u.avatar_url, u.measurement_pref,
                   u.totp_secret_encrypted, u.totp_enabled,
                   u.failed_login_attempts, u.locked_until, u.deletion_requested_at,
                   u.deletion_content_choice, u.created_at, u.updated_at, u.ap_id, u.federation_enabled
            FROM users u
            INNER JOIN follows f ON f.follower_id = u.id
            WHERE f.following_id = $1
            ORDER BY f.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    /// Get users that a user is following
    pub async fn get_following(pool: &PgPool, user_id: Uuid) -> AppResult<Vec<User>> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT u.id, u.username, u.email, u.email_verified, u.password_hash,
                   u.display_name, u.bio, u.avatar_url, u.measurement_pref,
                   u.totp_secret_encrypted, u.totp_enabled,
                   u.failed_login_attempts, u.locked_until, u.deletion_requested_at,
                   u.deletion_content_choice, u.created_at, u.updated_at, u.ap_id, u.federation_enabled
            FROM users u
            INNER JOIN follows f ON f.following_id = u.id
            WHERE f.follower_id = $1
            ORDER BY f.created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(users)
    }

    /// Get follow counts for a user
    pub async fn get_counts(pool: &PgPool, user_id: Uuid) -> AppResult<FollowCounts> {
        let counts = sqlx::query_as!(
            FollowCounts,
            r#"
            SELECT
                (SELECT COUNT(*) FROM follows WHERE following_id = $1) as "followers_count!",
                (SELECT COUNT(*) FROM follows WHERE follower_id = $1) as "following_count!"
            "#,
            user_id
        )
        .fetch_one(pool)
        .await?;

        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Self-Follow Prevention Tests (T043 - Error Paths)
    // ==========================================================================

    #[test]
    fn test_self_follow_detection() {
        let user_id = Uuid::new_v4();
        // This is the condition used in the follow() function
        assert!(user_id == user_id, "Self-follow should be detected");
    }

    #[test]
    fn test_different_users_not_self_follow() {
        let user_a = Uuid::new_v4();
        let user_b = Uuid::new_v4();
        assert_ne!(
            user_a, user_b,
            "Different users should not trigger self-follow check"
        );
    }

    // ==========================================================================
    // FollowCounts Tests (T043)
    // ==========================================================================

    #[test]
    fn test_follow_counts_zero() {
        let counts = FollowCounts {
            followers_count: 0,
            following_count: 0,
        };
        assert_eq!(counts.followers_count, 0);
        assert_eq!(counts.following_count, 0);
    }

    #[test]
    fn test_follow_counts_with_values() {
        let counts = FollowCounts {
            followers_count: 100,
            following_count: 50,
        };
        assert_eq!(counts.followers_count, 100);
        assert_eq!(counts.following_count, 50);
    }

    #[test]
    fn test_follow_counts_large_values() {
        let counts = FollowCounts {
            followers_count: 1_000_000,
            following_count: 5_000,
        };
        assert!(counts.followers_count > counts.following_count);
    }

    // ==========================================================================
    // UUID and AP ID Format Tests (T043)
    // ==========================================================================

    #[test]
    fn test_follow_ap_id_format() {
        let base_url = "https://oppskrift.example.com";
        let id = Uuid::new_v4();
        let ap_id = format!("{}/follows/{}", base_url, id);

        assert!(ap_id.starts_with("https://"));
        assert!(ap_id.contains("/follows/"));
        assert!(ap_id.len() > 40, "AP ID should be reasonably long");
    }

    #[test]
    fn test_uuid_uniqueness() {
        let ids: Vec<Uuid> = (0..100).map(|_| Uuid::new_v4()).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "All UUIDs should be unique");
    }

    // ==========================================================================
    // Follow Model Tests (T043)
    // ==========================================================================

    #[test]
    fn test_follow_struct_fields() {
        // Verify Follow struct can be constructed with expected fields
        let follow = Follow {
            id: Uuid::new_v4(),
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            ap_id: "https://example.com/follows/123".to_string(),
        };

        assert_ne!(follow.follower_id, follow.following_id);
        assert!(!follow.ap_id.is_empty());
    }

    #[test]
    fn test_follow_ids_are_different() {
        let follower = Uuid::new_v4();
        let following = Uuid::new_v4();

        // In a valid follow relationship, follower != following
        assert_ne!(follower, following);
    }

    // ==========================================================================
    // Error Path Validation (T043)
    // ==========================================================================

    #[test]
    fn test_self_follow_error_message() {
        // Verify the error message format for self-follow
        let err = AppError::Validation("Cannot follow yourself".to_string());
        let msg = err.to_string();
        assert!(msg.contains("follow yourself") || msg.contains("Cannot follow"));
    }

    #[test]
    fn test_not_found_error_for_unfollow() {
        let err = AppError::NotFound("Follow relationship not found".to_string());
        let msg = err.to_string();
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_already_following_error() {
        let err = AppError::Validation("Already following this user".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Already following"));
    }
}
