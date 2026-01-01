//! Book contribution model for collaborative book editing
//!
//! Tracks which recipes were added to a book by contributors.
//! Recipe ownership remains with the original author.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Contribution status for workflow (T006)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ContributionStatus {
    Pending,
    #[default]
    Accepted,
    Rejected,
}

impl ContributionStatus {
    /// Create from database string value
    pub fn from_db_str(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "accepted" => Self::Accepted,
            "rejected" => Self::Rejected,
            _ => Self::Accepted, // Default for backwards compat
        }
    }

    /// Convert to database string value
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }

    /// Human-readable display label
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Pending => "Pending Review",
            Self::Accepted => "Accepted",
            Self::Rejected => "Rejected",
        }
    }

    /// CSS class for status badge styling
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Pending => "badge-warning",
            Self::Accepted => "badge-success",
            Self::Rejected => "badge-error",
        }
    }
}

/// Book contribution entity - tracks contributed recipes (T006)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookContribution {
    pub id: Uuid,
    pub book_id: Uuid,
    pub recipe_id: Uuid,
    pub contributor_id: Uuid,
    pub added_at: DateTime<Utc>,
    #[sqlx(default)]
    pub status: String,
    pub rejection_reason: Option<String>,
}

impl BookContribution {
    /// Get typed status
    pub fn contribution_status(&self) -> ContributionStatus {
        ContributionStatus::from_db_str(&self.status)
    }

    /// Check if contribution is visible in book (only accepted)
    pub fn is_visible(&self) -> bool {
        self.contribution_status() == ContributionStatus::Accepted
    }
}

/// Book contribution with display information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookContributionWithDisplay {
    pub id: Uuid,
    pub book_id: Uuid,
    pub recipe_id: Uuid,
    pub contributor_id: Uuid,
    pub contributor_display_name: String,
    pub added_at: DateTime<Utc>,
    pub status: String,
    pub rejection_reason: Option<String>,
}

impl BookContributionWithDisplay {
    /// Get typed status
    pub fn contribution_status(&self) -> ContributionStatus {
        ContributionStatus::from_db_str(&self.status)
    }
}

/// Request to add a recipe contribution to a book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddContributionRequest {
    pub recipe_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_contribution_request_serialization() {
        let recipe_id = Uuid::new_v4();
        let req = AddContributionRequest { recipe_id };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: AddContributionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.recipe_id, recipe_id);
    }

    #[test]
    fn test_book_contribution_with_display_serialization() {
        let contribution = BookContributionWithDisplay {
            id: Uuid::new_v4(),
            book_id: Uuid::new_v4(),
            recipe_id: Uuid::new_v4(),
            contributor_id: Uuid::new_v4(),
            contributor_display_name: "John Doe".to_string(),
            added_at: Utc::now(),
            status: "accepted".to_string(),
            rejection_reason: None,
        };
        let json = serde_json::to_string(&contribution).unwrap();
        assert!(json.contains("John Doe"));
    }

    #[test]
    fn test_contribution_status_from_str() {
        assert_eq!(
            ContributionStatus::from_db_str("pending"),
            ContributionStatus::Pending
        );
        assert_eq!(
            ContributionStatus::from_db_str("accepted"),
            ContributionStatus::Accepted
        );
        assert_eq!(
            ContributionStatus::from_db_str("rejected"),
            ContributionStatus::Rejected
        );
        assert_eq!(
            ContributionStatus::from_db_str("unknown"),
            ContributionStatus::Accepted
        );
    }

    #[test]
    fn test_contribution_status_as_str() {
        assert_eq!(ContributionStatus::Pending.as_str(), "pending");
        assert_eq!(ContributionStatus::Accepted.as_str(), "accepted");
        assert_eq!(ContributionStatus::Rejected.as_str(), "rejected");
    }

    #[test]
    fn test_contribution_status_display() {
        assert_eq!(
            ContributionStatus::Pending.display_label(),
            "Pending Review"
        );
        assert_eq!(ContributionStatus::Pending.css_class(), "badge-warning");
    }
}
