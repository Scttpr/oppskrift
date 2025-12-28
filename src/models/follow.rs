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

/// Follow counts for a user profile
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct FollowCounts {
    pub followers_count: i64,
    pub following_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // Follow Serialization Tests (T050)
    // ==========================================================================

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

    #[test]
    fn test_follow_serialization_roundtrip() {
        let follow = Follow {
            id: Uuid::new_v4(),
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/follows/456".to_string(),
        };

        let json = serde_json::to_string(&follow).unwrap();
        let deserialized: Follow = serde_json::from_str(&json).unwrap();

        assert_eq!(follow.id, deserialized.id);
        assert_eq!(follow.follower_id, deserialized.follower_id);
        assert_eq!(follow.following_id, deserialized.following_id);
        assert_eq!(follow.ap_id, deserialized.ap_id);
    }

    #[test]
    fn test_follow_json_contains_all_fields() {
        let follow = Follow {
            id: Uuid::new_v4(),
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/follows/test".to_string(),
        };

        let json = serde_json::to_string(&follow).unwrap();
        assert!(json.contains("\"id\":"));
        assert!(json.contains("\"follower_id\":"));
        assert!(json.contains("\"following_id\":"));
        assert!(json.contains("\"created_at\":"));
        assert!(json.contains("\"ap_id\":"));
    }

    // ==========================================================================
    // FollowCounts Tests (T050)
    // ==========================================================================

    #[test]
    fn test_follow_counts_serialization() {
        let counts = FollowCounts {
            followers_count: 100,
            following_count: 50,
        };

        let json = serde_json::to_string(&counts).unwrap();
        assert!(json.contains("\"followers_count\":100"));
        assert!(json.contains("\"following_count\":50"));
    }

    #[test]
    fn test_follow_counts_roundtrip() {
        let counts = FollowCounts {
            followers_count: 1000,
            following_count: 250,
        };

        let json = serde_json::to_string(&counts).unwrap();
        let deserialized: FollowCounts = serde_json::from_str(&json).unwrap();

        assert_eq!(counts.followers_count, deserialized.followers_count);
        assert_eq!(counts.following_count, deserialized.following_count);
    }

    #[test]
    fn test_follow_counts_zero() {
        let counts = FollowCounts {
            followers_count: 0,
            following_count: 0,
        };

        let json = serde_json::to_string(&counts).unwrap();
        let deserialized: FollowCounts = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.followers_count, 0);
        assert_eq!(deserialized.following_count, 0);
    }

    #[test]
    fn test_follow_counts_large_numbers() {
        let counts = FollowCounts {
            followers_count: i64::MAX,
            following_count: i64::MAX,
        };

        let json = serde_json::to_string(&counts).unwrap();
        let deserialized: FollowCounts = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.followers_count, i64::MAX);
        assert_eq!(deserialized.following_count, i64::MAX);
    }

    #[test]
    fn test_follow_counts_asymmetric() {
        // Someone with many followers but following few
        let popular = FollowCounts {
            followers_count: 10000,
            following_count: 100,
        };
        assert!(popular.followers_count > popular.following_count);

        // Someone following many but few followers
        let active = FollowCounts {
            followers_count: 50,
            following_count: 500,
        };
        assert!(active.following_count > active.followers_count);
    }

    // ==========================================================================
    // UUID Uniqueness Tests (T050)
    // ==========================================================================

    #[test]
    fn test_follow_uuid_uniqueness() {
        let follow1 = Follow {
            id: Uuid::new_v4(),
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/1".to_string(),
        };

        let follow2 = Follow {
            id: Uuid::new_v4(),
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/2".to_string(),
        };

        assert_ne!(follow1.id, follow2.id);
        assert_ne!(follow1.follower_id, follow2.follower_id);
        assert_ne!(follow1.following_id, follow2.following_id);
    }

    #[test]
    fn test_follow_different_users() {
        let follower = Uuid::new_v4();
        let following = Uuid::new_v4();

        let follow = Follow {
            id: Uuid::new_v4(),
            follower_id: follower,
            following_id: following,
            created_at: Utc::now(),
            ap_id: "https://example.com/follow".to_string(),
        };

        assert_ne!(follow.follower_id, follow.following_id);
    }

    // ==========================================================================
    // AP ID Format Tests (T050)
    // ==========================================================================

    #[test]
    fn test_follow_ap_id_format() {
        let base_url = "https://oppskrift.example.com";
        let id = Uuid::new_v4();
        let ap_id = format!("{}/follows/{}", base_url, id);

        let follow = Follow {
            id,
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: ap_id.clone(),
        };

        assert!(follow.ap_id.starts_with(base_url));
        assert!(follow.ap_id.contains("/follows/"));
        assert!(follow.ap_id.contains(&id.to_string()));
    }

    #[test]
    fn test_follow_ap_id_contains_https() {
        let follow = Follow {
            id: Uuid::new_v4(),
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://secure.example.com/follows/123".to_string(),
        };

        assert!(follow.ap_id.starts_with("https://"));
    }

    // ==========================================================================
    // Clone and Debug Tests (T050)
    // ==========================================================================

    #[test]
    fn test_follow_clone() {
        let original = Follow {
            id: Uuid::new_v4(),
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/clone".to_string(),
        };

        let cloned = original.clone();
        assert_eq!(original.id, cloned.id);
        assert_eq!(original.follower_id, cloned.follower_id);
        assert_eq!(original.following_id, cloned.following_id);
        assert_eq!(original.ap_id, cloned.ap_id);
    }

    #[test]
    fn test_follow_counts_clone() {
        let original = FollowCounts {
            followers_count: 42,
            following_count: 24,
        };

        let cloned = original.clone();
        assert_eq!(original.followers_count, cloned.followers_count);
        assert_eq!(original.following_count, cloned.following_count);
    }

    #[test]
    fn test_follow_debug() {
        let follow = Follow {
            id: Uuid::new_v4(),
            follower_id: Uuid::new_v4(),
            following_id: Uuid::new_v4(),
            created_at: Utc::now(),
            ap_id: "https://example.com/debug".to_string(),
        };

        let debug_str = format!("{:?}", follow);
        assert!(debug_str.contains("Follow"));
        assert!(debug_str.contains("follower_id"));
        assert!(debug_str.contains("following_id"));
    }
}
