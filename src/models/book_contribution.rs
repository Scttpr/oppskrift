//! Book contribution model for collaborative book editing
//!
//! Tracks which recipes were added to a book by contributors.
//! Recipe ownership remains with the original author.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Book contribution entity - tracks contributed recipes
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookContribution {
    pub id: Uuid,
    pub book_id: Uuid,
    pub recipe_id: Uuid,
    pub contributor_id: Uuid,
    pub added_at: DateTime<Utc>,
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
        };
        let json = serde_json::to_string(&contribution).unwrap();
        assert!(json.contains("John Doe"));
    }
}
