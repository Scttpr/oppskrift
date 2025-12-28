use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Ingredient entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Ingredient {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub position: i32,
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    pub name: String,
    pub notes: Option<String>,
}

/// Create a new ingredient
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateIngredient {
    #[validate(range(min = 1, max = 50, message = "Position must be 1-50"))]
    pub position: i32,
    pub quantity: Option<Decimal>,
    #[validate(length(max = 50, message = "Unit must be at most 50 characters"))]
    pub unit: Option<String>,
    #[validate(length(min = 1, max = 200, message = "Name must be 1-200 characters"))]
    pub name: String,
    #[validate(length(max = 500, message = "Notes must be at most 500 characters"))]
    pub notes: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_ingredient() {
        let input = CreateIngredient {
            position: 1,
            quantity: Some(dec!(250.0)),
            unit: Some("g".to_string()),
            name: "flour".to_string(),
            notes: Some("all-purpose".to_string()),
        };

        assert_eq!(input.name, "flour");
        assert_eq!(input.quantity, Some(dec!(250.0)));
    }
}
