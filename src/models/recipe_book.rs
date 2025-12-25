use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::recipe::Visibility;

/// Recipe book entity - collection of recipes
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecipeBook {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    pub visibility: Visibility,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ap_id: String,
}

/// Create a new recipe book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecipeBook {
    pub title: String,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    pub visibility: Option<Visibility>,
}

/// Update a recipe book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecipeBook {
    pub title: Option<String>,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    pub visibility: Option<Visibility>,
}

/// Recipe book summary for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeBookSummary {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    pub visibility: Visibility,
    pub created_at: DateTime<Utc>,
    pub recipe_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_recipe_book() {
        let input = CreateRecipeBook {
            title: "Italian Classics".to_string(),
            description: Some("My favorite Italian recipes".to_string()),
            cover_image_url: None,
            visibility: Some(Visibility::Public),
        };

        assert_eq!(input.title, "Italian Classics");
    }
}
