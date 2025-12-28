use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::core::pagination::{PaginatedResponse, PaginationParams};
use crate::models::{Activity, ActivityType, ActivityWithActor, CreateActivity, TargetType};

pub struct ActivityService;

impl ActivityService {
    /// Create a new activity
    pub async fn create(
        pool: &PgPool,
        input: CreateActivity,
        base_url: &str,
    ) -> AppResult<Activity> {
        let id = Uuid::new_v4();
        let ap_id = format!("{}/activities/{}", base_url, id);

        let activity = sqlx::query_as!(
            Activity,
            r#"
            INSERT INTO activities (id, actor_id, activity_type, target_type, target_id, ap_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id, actor_id,
                activity_type as "activity_type: ActivityType",
                target_type as "target_type: TargetType",
                target_id, created_at, ap_id
            "#,
            id,
            input.actor_id,
            input.activity_type as ActivityType,
            input.target_type as TargetType,
            input.target_id,
            ap_id
        )
        .fetch_one(pool)
        .await?;

        Ok(activity)
    }

    /// Create activity for recipe creation
    pub async fn create_recipe_activity(
        pool: &PgPool,
        author_id: Uuid,
        recipe_id: Uuid,
        base_url: &str,
    ) -> AppResult<Activity> {
        Self::create(
            pool,
            CreateActivity {
                actor_id: author_id,
                activity_type: ActivityType::Create,
                target_type: TargetType::Recipe,
                target_id: recipe_id,
            },
            base_url,
        )
        .await
    }

    /// Create activity for book creation
    pub async fn create_book_activity(
        pool: &PgPool,
        owner_id: Uuid,
        book_id: Uuid,
        base_url: &str,
    ) -> AppResult<Activity> {
        Self::create(
            pool,
            CreateActivity {
                actor_id: owner_id,
                activity_type: ActivityType::Create,
                target_type: TargetType::Book,
                target_id: book_id,
            },
            base_url,
        )
        .await
    }

    /// Create activity for following a user
    pub async fn create_follow_activity(
        pool: &PgPool,
        follower_id: Uuid,
        following_id: Uuid,
        base_url: &str,
    ) -> AppResult<Activity> {
        Self::create(
            pool,
            CreateActivity {
                actor_id: follower_id,
                activity_type: ActivityType::Follow,
                target_type: TargetType::User,
                target_id: following_id,
            },
            base_url,
        )
        .await
    }

    /// Share a recipe (Announce activity)
    pub async fn share_recipe(
        pool: &PgPool,
        actor_id: Uuid,
        recipe_id: Uuid,
        base_url: &str,
    ) -> AppResult<Activity> {
        Self::create(
            pool,
            CreateActivity {
                actor_id,
                activity_type: ActivityType::Share,
                target_type: TargetType::Recipe,
                target_id: recipe_id,
            },
            base_url,
        )
        .await
    }

    /// Get activity feed for a user (activities from followed users)
    pub async fn get_feed(
        pool: &PgPool,
        user_id: Uuid,
        params: &PaginationParams,
    ) -> AppResult<PaginatedResponse<ActivityWithActor>> {
        let limit = params.limit();
        let offset = params.offset();

        // Get total count of activities from followed users
        let total: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM activities a
            WHERE a.actor_id IN (
                SELECT following_id FROM follows WHERE follower_id = $1
            )
            "#,
            user_id
        )
        .fetch_one(pool)
        .await?;

        // Get paginated activities with actor info
        let activities = sqlx::query_as!(
            ActivityWithActor,
            r#"
            SELECT
                a.id,
                a.actor_id,
                u.username as actor_username,
                u.display_name as actor_display_name,
                u.avatar_url as actor_avatar_url,
                a.activity_type as "activity_type: ActivityType",
                a.target_type as "target_type: TargetType",
                a.target_id,
                a.created_at
            FROM activities a
            INNER JOIN users u ON u.id = a.actor_id
            WHERE a.actor_id IN (
                SELECT following_id FROM follows WHERE follower_id = $1
            )
            ORDER BY a.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?;

        Ok(PaginatedResponse::new(
            activities,
            params.page,
            limit,
            total as u64,
        ))
    }

    /// Get activities by a specific user
    pub async fn get_user_activities(
        pool: &PgPool,
        user_id: Uuid,
        params: &PaginationParams,
    ) -> AppResult<PaginatedResponse<Activity>> {
        let limit = params.limit();
        let offset = params.offset();

        let total: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM activities
            WHERE actor_id = $1
            "#,
            user_id
        )
        .fetch_one(pool)
        .await?;

        let activities = sqlx::query_as!(
            Activity,
            r#"
            SELECT
                id, actor_id,
                activity_type as "activity_type: ActivityType",
                target_type as "target_type: TargetType",
                target_id, created_at, ap_id
            FROM activities
            WHERE actor_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit as i64,
            offset as i64
        )
        .fetch_all(pool)
        .await?;

        Ok(PaginatedResponse::new(
            activities,
            params.page,
            limit,
            total as u64,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_type_mapping() {
        let create = ActivityType::Create;
        let share = ActivityType::Share;
        let follow = ActivityType::Follow;

        assert_eq!(create.to_string(), "create");
        assert_eq!(share.to_string(), "share");
        assert_eq!(follow.to_string(), "follow");
    }
}
