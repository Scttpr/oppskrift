use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Recipe image entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecipeImage {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub url: String,
    pub alt_text: Option<String>,
    pub position: i32,
    pub is_primary: bool,
}

/// Create a new recipe image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRecipeImage {
    pub url: String,
    pub alt_text: Option<String>,
    pub position: i32,
    pub is_primary: bool,
}

/// Update a recipe image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRecipeImage {
    pub alt_text: Option<String>,
    pub position: Option<i32>,
    pub is_primary: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_recipe_image() {
        let image = CreateRecipeImage {
            url: "https://example.com/image.jpg".to_string(),
            alt_text: Some("A delicious pasta dish".to_string()),
            position: 1,
            is_primary: true,
        };

        assert!(image.is_primary);
        assert_eq!(image.position, 1);
    }
}
