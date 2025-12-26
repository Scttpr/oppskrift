use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

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
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateRecipeBook {
    #[validate(length(min = 1, max = 200, message = "Title must be 1-200 characters"))]
    pub title: String,
    #[validate(length(max = 1000, message = "Description must be at most 1000 characters"))]
    pub description: Option<String>,
    #[validate(url(message = "Cover image must be a valid URL"))]
    pub cover_image_url: Option<String>,
    pub visibility: Option<Visibility>,
}

/// Update a recipe book
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateRecipeBook {
    #[validate(length(min = 1, max = 200, message = "Title must be 1-200 characters"))]
    pub title: Option<String>,
    #[validate(length(max = 1000, message = "Description must be at most 1000 characters"))]
    pub description: Option<String>,
    #[validate(url(message = "Cover image must be a valid URL"))]
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
