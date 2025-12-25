use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Follow relationship between users
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Follow {
    pub id: Uuid,
    pub follower_id: Uuid,
    pub following_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub ap_id: String,
}

/// Input for creating a follow relationship
#[derive(Debug, Clone, Deserialize)]
pub struct CreateFollow {
    pub following_id: Uuid,
}

/// Follow counts for a user profile
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FollowCounts {
    pub followers_count: i64,
    pub following_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_follow_serialization() {
        let follow = Follow {
            id: Uuid::new_v4(),
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/follows/123".to_string(),
        };

        let json = serde_json::to_string(&follow).unwrap();
        assert!(json.contains("follower_id"));
        assert!(json.contains("following_id"));
    }
}
