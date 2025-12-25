use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIngredient {
    pub position: i32,
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    pub name: String,
    pub notes: Option<String>,
}

/// Update an ingredient
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIngredient {
    pub position: Option<i32>,
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    pub name: Option<String>,
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
