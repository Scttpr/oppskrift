//! Group models for batch permission management
//!
//! Groups allow sharing resources with multiple users at once.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Group entity - a named collection of users
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Group with additional computed fields for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupWithMeta {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub member_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_owner: bool,
}

/// Group detail with members list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupDetail {
    #[serde(flatten)]
    pub group: GroupWithMeta,
    pub members: Vec<GroupMemberInfo>,
}

/// Request to create a new group
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateGroupRequest {
    #[validate(length(min = 1, max = 100, message = "Group name must be 1-100 characters"))]
    pub name: String,
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
}

/// Request to update a group
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateGroupRequest {
    #[validate(length(min = 1, max = 100, message = "Group name must be 1-100 characters"))]
    pub name: Option<String>,
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
}

/// Group member association
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupMember {
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub added_at: DateTime<Utc>,
    pub added_by: Option<Uuid>,
}

/// Group member with user information for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMemberInfo {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub added_at: DateTime<Utc>,
    pub added_by: Option<Uuid>,
}

/// Request to add a member to a group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
}

/// Response for group list API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupListResponse {
    pub groups: Vec<GroupWithMeta>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

/// Response for member list API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberListResponse {
    pub members: Vec<GroupMemberInfo>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

/// Filter for listing groups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GroupFilter {
    #[default]
    All,
    Owned,
    Member,
}

impl std::fmt::Display for GroupFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupFilter::All => write!(f, "all"),
            GroupFilter::Owned => write!(f, "owned"),
            GroupFilter::Member => write!(f, "member"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_create_group_valid() {
        let req = CreateGroupRequest {
            name: "Family".to_string(),
            description: Some("Family recipes".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_group_minimal() {
        let req = CreateGroupRequest {
            name: "Friends".to_string(),
            description: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_group_name_empty() {
        let req = CreateGroupRequest {
            name: "".to_string(),
            description: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_group_name_too_long() {
        let req = CreateGroupRequest {
            name: "x".repeat(101),
            description: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_group_description_too_long() {
        let req = CreateGroupRequest {
            name: "Test".to_string(),
            description: Some("x".repeat(501)),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_update_group_valid() {
        let req = UpdateGroupRequest {
            name: Some("New Name".to_string()),
            description: Some("New description".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_group_empty() {
        let req = UpdateGroupRequest {
            name: None,
            description: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_group_filter_default() {
        assert_eq!(GroupFilter::default(), GroupFilter::All);
    }

    #[test]
    fn test_group_filter_serialization() {
        assert_eq!(serde_json::to_string(&GroupFilter::All).unwrap(), "\"all\"");
        assert_eq!(
            serde_json::to_string(&GroupFilter::Owned).unwrap(),
            "\"owned\""
        );
        assert_eq!(
            serde_json::to_string(&GroupFilter::Member).unwrap(),
            "\"member\""
        );
    }
}
