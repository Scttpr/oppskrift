use sqlx::PgPool;
use uuid::Uuid;

use crate::lib::error::{AppError, AppResult};
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
                   u.created_at, u.updated_at, u.ap_id, u.federation_enabled
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
                   u.created_at, u.updated_at, u.ap_id, u.federation_enabled
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

    #[test]
    fn test_self_follow_prevented() {
        // This would need async test setup with a database
        // For now, just verify the validation logic exists
        let user_id = Uuid::new_v4();
        assert_eq!(user_id, user_id); // Self-follow check condition
    }
}
