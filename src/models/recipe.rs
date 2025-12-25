use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Recipe visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "visibility_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Self::Public
    }
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Public => write!(f, "Public"),
            Visibility::Private => write!(f, "Private"),
        }
    }
}

/// Recipe difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "difficulty_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Difficulty::Easy => write!(f, "Easy"),
            Difficulty::Medium => write!(f, "Medium"),
            Difficulty::Hard => write!(f, "Hard"),
        }
    }
}

/// Recipe entity - core content type
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Recipe {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub servings: Option<String>,
    pub difficulty: Option<Difficulty>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ap_id: String,
}

/// Create a new recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecipe {
    pub title: String,
    pub description: Option<String>,
    pub visibility: Option<Visibility>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub servings: Option<String>,
    pub difficulty: Option<Difficulty>,
}

/// Update an existing recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecipe {
    pub title: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<Visibility>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub servings: Option<String>,
    pub difficulty: Option<Difficulty>,
}

/// Recipe summary for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSummary {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub difficulty: Option<Difficulty>,
    pub created_at: DateTime<Utc>,
    pub primary_image_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Public);
    }

    #[test]
    fn test_difficulty_serialization() {
        let easy = Difficulty::Easy;
        let json = serde_json::to_string(&easy).unwrap();
        assert_eq!(json, "\"easy\"");
    }
}
