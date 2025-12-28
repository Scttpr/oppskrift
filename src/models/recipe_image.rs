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
