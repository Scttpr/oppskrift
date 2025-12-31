//! Permission model for ABAC authorization system
//!
//! Defines permission levels, subject types, and the Permission entity
//! for managing resource access control.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Permission level defines the access rights granted
///
/// Hierarchy: Edit > Contributor > View
/// When a user has multiple permission paths, the highest level applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "permission_level", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    /// Read-only access to the resource
    View,
    /// Can modify the resource content
    Edit,
    /// Can add own recipes to a book (books only)
    Contributor,
}

impl PermissionLevel {
    /// Returns the numeric rank for permission comparison
    /// Higher rank means more permissions
    pub fn rank(&self) -> u8 {
        match self {
            PermissionLevel::View => 1,
            PermissionLevel::Contributor => 2,
            PermissionLevel::Edit => 3,
        }
    }

    /// Returns true if self grants at least the required permission level
    pub fn grants(&self, required: PermissionLevel) -> bool {
        self.rank() >= required.rank()
    }
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionLevel::View => write!(f, "View"),
            PermissionLevel::Edit => write!(f, "Edit"),
            PermissionLevel::Contributor => write!(f, "Contributor"),
        }
    }
}

/// Subject type defines who can receive permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "subject_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubjectType {
    /// A specific user (local or federated)
    User,
    /// A group of users
    Group,
    /// All users from a federated instance
    Instance,
}

impl std::fmt::Display for SubjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubjectType::User => write!(f, "User"),
            SubjectType::Group => write!(f, "Group"),
            SubjectType::Instance => write!(f, "Instance"),
        }
    }
}

/// Resource type for permission grants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Recipe,
    Book,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Recipe => "recipe",
            ResourceType::Book => "book",
        }
    }
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Permission entity - represents a granted permission on a resource
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Permission {
    pub id: Uuid,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub subject_type: SubjectType,
    pub subject_id: Option<Uuid>,
    pub subject_domain: Option<String>,
    pub permission_level: PermissionLevel,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
}

/// Request to grant a permission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantPermissionRequest {
    pub subject_type: SubjectType,
    pub subject_id: Option<Uuid>,
    pub subject_domain: Option<String>,
    pub permission_level: PermissionLevel,
}

impl GrantPermissionRequest {
    /// Validate the request based on subject type
    pub fn validate(&self) -> Result<(), &'static str> {
        match self.subject_type {
            SubjectType::User | SubjectType::Group => {
                if self.subject_id.is_none() {
                    return Err("subject_id is required for user and group permissions");
                }
                if self.subject_domain.is_some() {
                    return Err("subject_domain must be null for user and group permissions");
                }
            }
            SubjectType::Instance => {
                if self.subject_id.is_some() {
                    return Err("subject_id must be null for instance permissions");
                }
                if self.subject_domain.is_none() {
                    return Err("subject_domain is required for instance permissions");
                }
            }
        }
        Ok(())
    }
}

/// Permission with display information for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionWithDisplay {
    pub id: Uuid,
    pub resource_type: String,
    pub resource_id: Uuid,
    pub subject_type: SubjectType,
    pub subject_id: Option<Uuid>,
    pub subject_domain: Option<String>,
    pub subject_display_name: String,
    pub permission_level: PermissionLevel,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
}

/// Response containing list of permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionListResponse {
    pub permissions: Vec<PermissionWithDisplay>,
    pub resource_type: String,
    pub resource_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_level_rank() {
        assert_eq!(PermissionLevel::View.rank(), 1);
        assert_eq!(PermissionLevel::Contributor.rank(), 2);
        assert_eq!(PermissionLevel::Edit.rank(), 3);
    }

    #[test]
    fn test_permission_level_grants() {
        // Edit grants everything
        assert!(PermissionLevel::Edit.grants(PermissionLevel::View));
        assert!(PermissionLevel::Edit.grants(PermissionLevel::Contributor));
        assert!(PermissionLevel::Edit.grants(PermissionLevel::Edit));

        // Contributor grants view and contributor
        assert!(PermissionLevel::Contributor.grants(PermissionLevel::View));
        assert!(PermissionLevel::Contributor.grants(PermissionLevel::Contributor));
        assert!(!PermissionLevel::Contributor.grants(PermissionLevel::Edit));

        // View only grants view
        assert!(PermissionLevel::View.grants(PermissionLevel::View));
        assert!(!PermissionLevel::View.grants(PermissionLevel::Contributor));
        assert!(!PermissionLevel::View.grants(PermissionLevel::Edit));
    }

    #[test]
    fn test_permission_level_display() {
        assert_eq!(PermissionLevel::View.to_string(), "View");
        assert_eq!(PermissionLevel::Edit.to_string(), "Edit");
        assert_eq!(PermissionLevel::Contributor.to_string(), "Contributor");
    }

    #[test]
    fn test_permission_level_serialization() {
        assert_eq!(
            serde_json::to_string(&PermissionLevel::View).unwrap(),
            "\"view\""
        );
        assert_eq!(
            serde_json::to_string(&PermissionLevel::Edit).unwrap(),
            "\"edit\""
        );
        assert_eq!(
            serde_json::to_string(&PermissionLevel::Contributor).unwrap(),
            "\"contributor\""
        );
    }

    #[test]
    fn test_subject_type_display() {
        assert_eq!(SubjectType::User.to_string(), "User");
        assert_eq!(SubjectType::Group.to_string(), "Group");
        assert_eq!(SubjectType::Instance.to_string(), "Instance");
    }

    #[test]
    fn test_subject_type_serialization() {
        assert_eq!(
            serde_json::to_string(&SubjectType::User).unwrap(),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&SubjectType::Group).unwrap(),
            "\"group\""
        );
        assert_eq!(
            serde_json::to_string(&SubjectType::Instance).unwrap(),
            "\"instance\""
        );
    }

    #[test]
    fn test_resource_type_as_str() {
        assert_eq!(ResourceType::Recipe.as_str(), "recipe");
        assert_eq!(ResourceType::Book.as_str(), "book");
    }

    #[test]
    fn test_grant_permission_request_valid_user() {
        let req = GrantPermissionRequest {
            subject_type: SubjectType::User,
            subject_id: Some(Uuid::new_v4()),
            subject_domain: None,
            permission_level: PermissionLevel::View,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_grant_permission_request_valid_group() {
        let req = GrantPermissionRequest {
            subject_type: SubjectType::Group,
            subject_id: Some(Uuid::new_v4()),
            subject_domain: None,
            permission_level: PermissionLevel::Edit,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_grant_permission_request_valid_instance() {
        let req = GrantPermissionRequest {
            subject_type: SubjectType::Instance,
            subject_id: None,
            subject_domain: Some("mastodon.social".to_string()),
            permission_level: PermissionLevel::View,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_grant_permission_request_invalid_user_no_id() {
        let req = GrantPermissionRequest {
            subject_type: SubjectType::User,
            subject_id: None,
            subject_domain: None,
            permission_level: PermissionLevel::View,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_grant_permission_request_invalid_user_with_domain() {
        let req = GrantPermissionRequest {
            subject_type: SubjectType::User,
            subject_id: Some(Uuid::new_v4()),
            subject_domain: Some("example.com".to_string()),
            permission_level: PermissionLevel::View,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_grant_permission_request_invalid_instance_with_id() {
        let req = GrantPermissionRequest {
            subject_type: SubjectType::Instance,
            subject_id: Some(Uuid::new_v4()),
            subject_domain: Some("example.com".to_string()),
            permission_level: PermissionLevel::View,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_grant_permission_request_invalid_instance_no_domain() {
        let req = GrantPermissionRequest {
            subject_type: SubjectType::Instance,
            subject_id: None,
            subject_domain: None,
            permission_level: PermissionLevel::View,
        };
        assert!(req.validate().is_err());
    }
}
