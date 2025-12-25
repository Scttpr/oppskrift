use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Book recipe entry - junction table for recipes in books
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BookRecipeEntry {
    pub id: Uuid,
    pub book_id: Uuid,
    pub recipe_id: Uuid,
    pub position: i32,
    pub added_at: DateTime<Utc>,
}

/// Add a recipe to a book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddRecipeToBook {
    pub recipe_id: Uuid,
    pub position: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_recipe_to_book() {
        let input = AddRecipeToBook {
            recipe_id: Uuid::new_v4(),
            position: Some(1),
        };

        assert_eq!(input.position, Some(1));
    }
}
