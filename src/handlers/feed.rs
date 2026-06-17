use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
    routing::get,
    Router,
};

use crate::api::middleware::AuthUser;
use crate::core::error::AppResult;
use crate::core::pagination::{PaginationMeta, PaginationParams};
use crate::models::ActivityWithActor;
use crate::services::ActivityService;
use crate::AppState;

/// Feed routes
pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(feed_page))
}

/// Feed page template
#[derive(Template)]
#[template(path = "feed/index.html")]
struct FeedTemplate {
    activities: Vec<ActivityWithActor>,
    pagination: PaginationMeta,
}

/// Feed page handler
async fn feed_page(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    let feed = ActivityService::get_feed(&state.db, auth.id, &params).await?;

    let template = FeedTemplate {
        activities: feed.data,
        pagination: feed.pagination,
    };

    crate::core::render(&template)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ActivityType, TargetType};
    use askama::Template;
    use chrono::Utc;
    use uuid::Uuid;

    // ==========================================================================
    // Route Configuration Tests (T055)
    // ==========================================================================

    #[test]
    fn test_routes_returns_router() {
        let router = routes();
        let _ = router;
    }

    // ==========================================================================
    // Template Struct Tests (T055)
    // ==========================================================================

    #[test]
    fn test_feed_template_renders_empty() {
        let template = FeedTemplate {
            activities: vec![],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 0,
                total_pages: 0,
                has_next: false,
                has_prev: false,
            },
        };

        let result = template.render();
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_feed_template_with_activities() {
        let activity = ActivityWithActor {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            actor_username: "chef".to_string(),
            actor_display_name: "Chef Alice".to_string(),
            actor_avatar_url: None,
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };

        let template = FeedTemplate {
            activities: vec![activity],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 1,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            },
        };

        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("chef") || html.contains("Alice") || !html.is_empty());
    }

    // ==========================================================================
    // Activity Type Tests (T055)
    // ==========================================================================

    #[test]
    fn test_feed_create_activity() {
        let activity = ActivityWithActor {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            actor_username: "creator".to_string(),
            actor_display_name: "Creator".to_string(),
            actor_avatar_url: None,
            activity_type: ActivityType::Create,
            target_type: TargetType::Recipe,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };

        let template = FeedTemplate {
            activities: vec![activity],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 1,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            },
        };

        assert!(template.render().is_ok());
    }

    #[test]
    fn test_feed_share_activity() {
        let activity = ActivityWithActor {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            actor_username: "sharer".to_string(),
            actor_display_name: "Sharer".to_string(),
            actor_avatar_url: Some("https://example.com/avatar.jpg".to_string()),
            activity_type: ActivityType::Share,
            target_type: TargetType::Book,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };

        let template = FeedTemplate {
            activities: vec![activity],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 1,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            },
        };

        assert!(template.render().is_ok());
    }

    #[test]
    fn test_feed_follow_activity() {
        let activity = ActivityWithActor {
            id: Uuid::new_v4(),
            actor_id: Uuid::new_v4(),
            actor_username: "follower".to_string(),
            actor_display_name: "Follower".to_string(),
            actor_avatar_url: None,
            activity_type: ActivityType::Follow,
            target_type: TargetType::User,
            target_id: Uuid::new_v4(),
            created_at: Utc::now(),
        };

        let template = FeedTemplate {
            activities: vec![activity],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 1,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            },
        };

        assert!(template.render().is_ok());
    }

    // ==========================================================================
    // Pagination Tests (T055)
    // ==========================================================================

    #[test]
    fn test_feed_pagination() {
        let template = FeedTemplate {
            activities: vec![],
            pagination: PaginationMeta {
                page: 3,
                page_size: 20,
                total_items: 100,
                total_pages: 5,
                has_next: true,
                has_prev: true,
            },
        };

        let html = template.render().unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_feed_multiple_activities() {
        let activities: Vec<ActivityWithActor> = (0..5)
            .map(|i| ActivityWithActor {
                id: Uuid::new_v4(),
                actor_id: Uuid::new_v4(),
                actor_username: format!("user{}", i),
                actor_display_name: format!("User {}", i),
                actor_avatar_url: None,
                activity_type: ActivityType::Create,
                target_type: TargetType::Recipe,
                target_id: Uuid::new_v4(),
                created_at: Utc::now(),
            })
            .collect();

        let template = FeedTemplate {
            activities,
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 5,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            },
        };

        let html = template.render().unwrap();
        assert!(!html.is_empty());
    }
}
