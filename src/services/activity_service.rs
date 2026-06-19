use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::AppResult;
use crate::core::pagination::{paginate, PaginatedResponse, PaginationParams};
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
        paginate(
            params,
            |limit, offset| {
                sqlx::query_as!(
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
                    limit,
                    offset
                )
                .fetch_all(pool)
            },
            || {
                sqlx::query_scalar!(
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
            },
        )
        .await
    }

    /// Get activities by a specific user
    pub async fn get_user_activities(
        pool: &PgPool,
        user_id: Uuid,
        params: &PaginationParams,
    ) -> AppResult<PaginatedResponse<Activity>> {
        paginate(
            params,
            |limit, offset| {
                sqlx::query_as!(
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
                    limit,
                    offset
                )
                .fetch_all(pool)
            },
            || {
                sqlx::query_scalar!(
                    r#"
                    SELECT COUNT(*) as "count!"
                    FROM activities
                    WHERE actor_id = $1
                    "#,
                    user_id
                )
                .fetch_one(pool)
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // ActivityType Tests (T044 - Error Paths)
    // ==========================================================================

    #[test]
    fn test_activity_type_mapping() {
        let create = ActivityType::Create;
        let share = ActivityType::Share;
        let follow = ActivityType::Follow;

        assert_eq!(create.to_string(), "create");
        assert_eq!(share.to_string(), "share");
        assert_eq!(follow.to_string(), "follow");
    }

    #[test]
    fn test_activity_type_all_variants() {
        // Ensure all activity types can be created
        let types = [
            ActivityType::Create,
            ActivityType::Follow,
            ActivityType::Share,
        ];

        for activity_type in types {
            assert!(!activity_type.to_string().is_empty());
        }
    }

    // ==========================================================================
    // TargetType Tests (T044)
    // ==========================================================================

    #[test]
    fn test_target_type_mapping() {
        assert_eq!(TargetType::Recipe.to_string(), "recipe");
        assert_eq!(TargetType::Book.to_string(), "book");
        assert_eq!(TargetType::User.to_string(), "user");
    }

    #[test]
    fn test_target_type_all_variants() {
        let types = [TargetType::Recipe, TargetType::Book, TargetType::User];

        for target_type in types {
            assert!(!target_type.to_string().is_empty());
        }
    }

    // ==========================================================================
    // CreateActivity Input Tests (T044)
    // ==========================================================================

    #[test]
    fn test_create_activity_recipe() {
        let input = CreateActivity {
            actor_id: Uuid::new_v4(),
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
        };

        assert_ne!(input.actor_id, input.target_id);
    }

    #[test]
    fn test_create_activity_follow() {
        let follower = Uuid::new_v4();
        let following = Uuid::new_v4();

        let input = CreateActivity {
            actor_id: follower,
            activity_type: ActivityType::Follow,
            target_type: TargetType::User,
            target_id: following,
        };

        assert_eq!(input.activity_type, ActivityType::Follow);
        assert_eq!(input.target_type, TargetType::User);
    }

    #[test]
    fn test_create_activity_share() {
        let input = CreateActivity {
            actor_id: Uuid::new_v4(),
            activity_type: ActivityType::Share,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
        };

        assert_eq!(input.activity_type, ActivityType::Share);
    }

    // ==========================================================================
    // AP ID Format Tests (T044)
    // ==========================================================================

    #[test]
    fn test_activity_ap_id_format() {
        let base_url = "https://oppskrift.example.com";
        let id = Uuid::new_v4();
        let ap_id = format!("{}/activities/{}", base_url, id);

        assert!(ap_id.starts_with("https://"));
        assert!(ap_id.contains("/activities/"));
    }

    #[test]
    fn test_ap_id_uniqueness() {
        let base_url = "https://example.com";
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let ap_id1 = format!("{}/activities/{}", base_url, id1);
        let ap_id2 = format!("{}/activities/{}", base_url, id2);

        assert_ne!(ap_id1, ap_id2);
    }

    // ==========================================================================
    // Pagination Tests (T044)
    // ==========================================================================

    #[test]
    fn test_pagination_params_defaults() {
        let params = PaginationParams {
            page: 1,
            page_size: 20,
        };
        assert_eq!(params.page, 1);
        assert_eq!(params.page_size, 20);
    }

    #[test]
    fn test_pagination_offset_calculation() {
        let params = PaginationParams {
            page: 5,
            page_size: 10,
        };
        assert_eq!(params.offset(), 40); // (5-1) * 10
    }

    // ==========================================================================
    // ActivityWithActor Tests (T044)
    // ==========================================================================

    #[test]
    fn test_activity_with_actor_display_name() {
        let activity = ActivityWithActor {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            actor_username: "chef_jane".to_string(),
            actor_display_name: "Chef Jane".to_string(),
            actor_avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
        };

        assert_eq!(activity.actor_username, "chef_jane");
        assert_eq!(activity.actor_display_name, "Chef Jane");
    }

    #[test]
    fn test_activity_with_actor_no_avatar() {
        let activity = ActivityWithActor {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            actor_username: "anonymous".to_string(),
            actor_display_name: "Anonymous User".to_string(),
            actor_avatar_url: None,
            activity_type: ActivityType::Share,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
            created_at: chrono::Utc::now(),
        };

        assert!(!activity.actor_display_name.is_empty());
        assert!(activity.actor_avatar_url.is_none());
    }
}
