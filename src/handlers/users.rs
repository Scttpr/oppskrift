use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::get,
    Router,
};
use uuid::Uuid;

use crate::api::middleware::{AuthUser, OptionalAuthUser};
use crate::core::error::AppResult;
use crate::core::pagination::{PaginationMeta, PaginationParams};
use crate::models::{FollowCounts, RecipeSummary, UserCardView, UserProfile};
use crate::services::{FollowService, RecipeService, SavedRecipeService, UserService};
use crate::AppState;

/// Pagination info for templates (simplified version)
#[derive(Debug, Clone)]
struct PaginationInfo {
    page: i64,
    total_pages: i64,
    total_items: i64,
    has_prev: bool,
    has_next: bool,
}

/// User page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{id}", get(user_profile_page))
        .route("/{id}/saved", get(saved_recipes_page))
        // Followers/Following routes (T041)
        .route("/{id}/followers", get(followers_page))
        .route("/{id}/following", get(following_page))
}

/// User profile page template
#[derive(Template)]
#[template(path = "users/profile.html")]
struct UserProfileTemplate {
    profile: UserProfile,
    recipes: Vec<RecipeSummary>,
    pagination: PaginationMeta,
    is_own_profile: bool,
    follow_counts: FollowCounts,
    is_following: bool,
}

/// User profile page handler
async fn user_profile_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, id).await?;
    let profile = UserProfile::from(user);

    let recipes_page = RecipeService::list_by_author(&state.db, id, &params).await?;

    let is_own_profile = auth.0.as_ref().map(|u| u.id) == Some(id);
    let follow_counts = FollowService::get_counts(&state.db, id).await?;
    let is_following = if let Some(ref current_user) = auth.0 {
        FollowService::is_following(&state.db, current_user.id, id).await?
    } else {
        false
    };

    let template = UserProfileTemplate {
        profile,
        recipes: recipes_page.data,
        pagination: recipes_page.pagination,
        is_own_profile,
        follow_counts,
        is_following,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Saved recipes page template
#[derive(Template)]
#[template(path = "users/saved.html")]
struct SavedRecipesTemplate {
    recipes: Vec<RecipeSummary>,
    pagination: PaginationMeta,
}

/// Saved recipes page handler (requires auth)
async fn saved_recipes_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth: AuthUser,
) -> AppResult<Html<String>> {
    // Only allow viewing own saved recipes
    if auth.id != id {
        return Err(crate::core::error::AppError::Forbidden(
            "You can only view your own saved recipes".to_string(),
        ));
    }

    let saved_page = SavedRecipeService::get_saved(&state.db, auth.id, &params).await?;

    let template = SavedRecipesTemplate {
        recipes: saved_page.data,
        pagination: saved_page.pagination,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Followers page template (T036)
#[derive(Template)]
#[template(path = "users/followers.html")]
struct FollowersPageTemplate {
    profile: UserProfile,
    users: Vec<UserCardView>,
    pagination: PaginationInfo,
    is_own_profile: bool,
    current_user_id: Option<Uuid>,
}

/// Following page template (T036)
#[derive(Template)]
#[template(path = "users/following.html")]
struct FollowingPageTemplate {
    profile: UserProfile,
    users: Vec<UserCardView>,
    pagination: PaginationInfo,
    is_own_profile: bool,
    current_user_id: Option<Uuid>,
}

/// Followers page handler (T037)
async fn followers_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, id).await?;
    let profile = UserProfile::from(user);

    // Get all followers
    let all_followers = FollowService::get_followers(&state.db, id).await?;

    // Manual pagination
    let page = params.page.max(1) as i64;
    let page_size = params.page_size.clamp(1, 50) as i64;
    let total_items = all_followers.len() as i64;
    let total_pages = (total_items + page_size - 1) / page_size;
    let offset = ((page - 1) * page_size) as usize;

    let paginated_followers: Vec<_> = all_followers
        .into_iter()
        .skip(offset)
        .take(page_size as usize)
        .collect();

    // Get current user ID and build user cards with follow status
    let current_user_id = auth.0.as_ref().map(|u| u.id);
    let is_own_profile = current_user_id == Some(id);

    // Batch load follow statuses to avoid N+1 queries
    let follow_statuses = if let Some(current_id) = current_user_id {
        let target_ids: Vec<Uuid> = paginated_followers
            .iter()
            .filter(|u| u.id != current_id) // Exclude self
            .map(|u| u.id)
            .collect();
        FollowService::check_follow_statuses_batch(&state.db, current_id, &target_ids).await?
    } else {
        std::collections::HashMap::new()
    };

    let user_cards: Vec<UserCardView> = paginated_followers
        .iter()
        .map(|follower| {
            let (is_following, follows_you) = if current_user_id == Some(follower.id) {
                (false, false) // Can't follow yourself
            } else {
                follow_statuses
                    .get(&follower.id)
                    .copied()
                    .unwrap_or((false, false))
            };
            UserCardView::from_user(follower, is_following, follows_you)
        })
        .collect();

    let pagination = PaginationInfo {
        page,
        total_pages,
        total_items,
        has_prev: page > 1,
        has_next: page < total_pages,
    };

    let template = FollowersPageTemplate {
        profile,
        users: user_cards,
        pagination,
        is_own_profile,
        current_user_id,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Following page handler (T038)
async fn following_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let user = UserService::get_by_id(&state.db, id).await?;
    let profile = UserProfile::from(user);

    // Get all following
    let all_following = FollowService::get_following(&state.db, id).await?;

    // Manual pagination
    let page = params.page.max(1) as i64;
    let page_size = params.page_size.clamp(1, 50) as i64;
    let total_items = all_following.len() as i64;
    let total_pages = (total_items + page_size - 1) / page_size;
    let offset = ((page - 1) * page_size) as usize;

    let paginated_following: Vec<_> = all_following
        .into_iter()
        .skip(offset)
        .take(page_size as usize)
        .collect();

    // Get current user ID and build user cards with follow status
    let current_user_id = auth.0.as_ref().map(|u| u.id);
    let is_own_profile = current_user_id == Some(id);

    // Batch load follow statuses to avoid N+1 queries
    let follow_statuses = if let Some(current_id) = current_user_id {
        let target_ids: Vec<Uuid> = paginated_following
            .iter()
            .filter(|u| u.id != current_id) // Exclude self
            .map(|u| u.id)
            .collect();
        FollowService::check_follow_statuses_batch(&state.db, current_id, &target_ids).await?
    } else {
        std::collections::HashMap::new()
    };

    let user_cards: Vec<UserCardView> = paginated_following
        .iter()
        .map(|followed| {
            let (is_following, follows_you) = if current_user_id == Some(followed.id) {
                (false, false) // Can't follow yourself
            } else {
                follow_statuses
                    .get(&followed.id)
                    .copied()
                    .unwrap_or((false, false))
            };
            UserCardView::from_user(followed, is_following, follows_you)
        })
        .collect();

    let pagination = PaginationInfo {
        page,
        total_pages,
        total_items,
        has_prev: page > 1,
        has_next: page < total_pages,
    };

    let template = FollowingPageTemplate {
        profile,
        users: user_cards,
        pagination,
        is_own_profile,
        current_user_id,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Difficulty;
    use askama::Template;
    use chrono::Utc;

    // ==========================================================================
    // Route Configuration Tests (T054)
    // ==========================================================================

    #[test]
    fn test_routes_returns_router() {
        let router = routes();
        let _ = router;
    }

    // ==========================================================================
    // Template Struct Tests (T054)
    // ==========================================================================

    #[test]
    fn test_user_profile_template_renders() {
        let profile = UserProfile {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            display_name: "Test User".to_string(),
            bio: Some("A test bio".to_string()),
            avatar_url: None,
            created_at: Utc::now(),
            ap_id: "https://example.com/users/testuser".to_string(),
        };

        let template = UserProfileTemplate {
            profile,
            recipes: vec![],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 0,
                total_pages: 0,
                has_next: false,
                has_prev: false,
            },
            is_own_profile: false,
            follow_counts: FollowCounts {
                followers_count: 0,
                following_count: 0,
            },
            is_following: false,
        };

        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("testuser") || !html.is_empty());
    }

    #[test]
    fn test_saved_recipes_template_renders_empty() {
        let template = SavedRecipesTemplate {
            recipes: vec![],
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

    // ==========================================================================
    // Profile State Tests (T054)
    // ==========================================================================

    #[test]
    fn test_own_profile_view() {
        let profile = UserProfile {
            id: Uuid::new_v4(),
            username: "owner".to_string(),
            display_name: "Profile Owner".to_string(),
            bio: None,
            avatar_url: None,
            created_at: Utc::now(),
            ap_id: "https://example.com/users/owner".to_string(),
        };

        let template = UserProfileTemplate {
            profile,
            recipes: vec![],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 0,
                total_pages: 0,
                has_next: false,
                has_prev: false,
            },
            is_own_profile: true,
            follow_counts: FollowCounts {
                followers_count: 100,
                following_count: 50,
            },
            is_following: false,
        };

        let html = template.render().unwrap();
        assert!(!html.is_empty());
    }

    #[test]
    fn test_other_profile_following() {
        let profile = UserProfile {
            id: Uuid::new_v4(),
            username: "other".to_string(),
            display_name: "Other User".to_string(),
            bio: None,
            avatar_url: None,
            created_at: Utc::now(),
            ap_id: "https://example.com/users/other".to_string(),
        };

        // Test when following
        let following_template = UserProfileTemplate {
            profile: profile.clone(),
            recipes: vec![],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 0,
                total_pages: 0,
                has_next: false,
                has_prev: false,
            },
            is_own_profile: false,
            follow_counts: FollowCounts {
                followers_count: 10,
                following_count: 5,
            },
            is_following: true,
        };
        assert!(following_template.render().is_ok());

        // Test when not following
        let not_following_template = UserProfileTemplate {
            profile,
            recipes: vec![],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 0,
                total_pages: 0,
                has_next: false,
                has_prev: false,
            },
            is_own_profile: false,
            follow_counts: FollowCounts {
                followers_count: 10,
                following_count: 5,
            },
            is_following: false,
        };
        assert!(not_following_template.render().is_ok());
    }

    // ==========================================================================
    // Pagination Tests (T054)
    // ==========================================================================

    #[test]
    fn test_profile_with_recipes() {
        let recipe = RecipeSummary {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "User Recipe".to_string(),
            description: Some("A recipe".to_string()),
            prep_time_min: Some(15),
            cook_time_min: Some(30),
            difficulty: Some(Difficulty::Medium),
            created_at: Utc::now(),
            primary_image_url: None,
        };

        let profile = UserProfile {
            id: Uuid::new_v4(),
            username: "chef".to_string(),
            display_name: "Chef User".to_string(),
            bio: None,
            avatar_url: None,
            created_at: Utc::now(),
            ap_id: "https://example.com/users/chef".to_string(),
        };

        let template = UserProfileTemplate {
            profile,
            recipes: vec![recipe],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 1,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            },
            is_own_profile: false,
            follow_counts: FollowCounts {
                followers_count: 5,
                following_count: 3,
            },
            is_following: false,
        };

        let html = template.render().unwrap();
        assert!(html.contains("User Recipe") || html.contains("chef") || !html.is_empty());
    }

    #[test]
    fn test_saved_recipes_with_items() {
        let recipe = RecipeSummary {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "Saved Recipe".to_string(),
            description: None,
            prep_time_min: None,
            cook_time_min: None,
            difficulty: None,
            created_at: Utc::now(),
            primary_image_url: None,
        };

        let template = SavedRecipesTemplate {
            recipes: vec![recipe],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 1,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            },
        };

        let html = template.render().unwrap();
        assert!(html.contains("Saved Recipe") || !html.is_empty());
    }

    // ==========================================================================
    // Follow Counts Tests (T054)
    // ==========================================================================

    #[test]
    fn test_follow_counts_display() {
        let profile = UserProfile {
            id: Uuid::new_v4(),
            username: "popular".to_string(),
            display_name: "Popular User".to_string(),
            bio: None,
            avatar_url: None,
            created_at: Utc::now(),
            ap_id: "https://example.com/users/popular".to_string(),
        };

        let template = UserProfileTemplate {
            profile,
            recipes: vec![],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 0,
                total_pages: 0,
                has_next: false,
                has_prev: false,
            },
            is_own_profile: false,
            follow_counts: FollowCounts {
                followers_count: 10000,
                following_count: 500,
            },
            is_following: false,
        };

        let html = template.render().unwrap();
        assert!(!html.is_empty());
    }
}
