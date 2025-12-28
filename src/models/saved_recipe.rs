use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A saved/bookmarked recipe (quick-save, not in a book)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SavedRecipe {
    pub id: Uuid,
    pub user_id: Uuid,
    pub recipe_id: Uuid,
    pub saved_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saved_recipe_serialization() {
        let saved = SavedRecipe {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recipe_id: Uuid::new_v4(),
            saved_at: Utc::now(),
        };

        let json = serde_json::to_string(&saved).unwrap();
        assert!(json.contains("user_id"));
        assert!(json.contains("recipe_id"));
    }
}
