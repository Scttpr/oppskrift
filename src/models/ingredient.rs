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
    use validator::Validate;

    // ==========================================================================
    // CreateIngredient Basic Tests (T047)
    // ==========================================================================

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

    #[test]
    fn test_create_ingredient_valid() {
        let input = CreateIngredient {
            position: 1,
            quantity: Some(dec!(2.5)),
            unit: Some("cups".to_string()),
            name: "Sugar".to_string(),
            notes: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_create_ingredient_minimal() {
        let input = CreateIngredient {
            position: 1,
            quantity: None,
            unit: None,
            name: "Salt".to_string(),
            notes: None,
        };
        assert!(input.validate().is_ok());
    }

    // ==========================================================================
    // Position Validation Tests (T047)
    // ==========================================================================

    #[test]
    fn test_ingredient_position_valid() {
        let input = CreateIngredient {
            position: 25,
            quantity: None,
            unit: None,
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_ingredient_position_min() {
        let input = CreateIngredient {
            position: 1,
            quantity: None,
            unit: None,
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_ingredient_position_max() {
        let input = CreateIngredient {
            position: 50,
            quantity: None,
            unit: None,
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_ingredient_position_zero() {
        let input = CreateIngredient {
            position: 0,
            quantity: None,
            unit: None,
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_ingredient_position_negative() {
        let input = CreateIngredient {
            position: -1,
            quantity: None,
            unit: None,
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_ingredient_position_too_high() {
        let input = CreateIngredient {
            position: 51,
            quantity: None,
            unit: None,
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_err());
    }

    // ==========================================================================
    // Name Validation Tests (T047)
    // ==========================================================================

    #[test]
    fn test_ingredient_name_empty() {
        let input = CreateIngredient {
            position: 1,
            quantity: None,
            unit: None,
            name: "".to_string(),
            notes: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_ingredient_name_too_long() {
        let input = CreateIngredient {
            position: 1,
            quantity: None,
            unit: None,
            name: "x".repeat(201),
            notes: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_ingredient_name_at_max() {
        let input = CreateIngredient {
            position: 1,
            quantity: None,
            unit: None,
            name: "x".repeat(200),
            notes: None,
        };
        assert!(input.validate().is_ok());
    }

    // ==========================================================================
    // Unit Validation Tests (T047)
    // ==========================================================================

    #[test]
    fn test_ingredient_unit_too_long() {
        let input = CreateIngredient {
            position: 1,
            quantity: None,
            unit: Some("x".repeat(51)),
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_ingredient_unit_at_max() {
        let input = CreateIngredient {
            position: 1,
            quantity: None,
            unit: Some("x".repeat(50)),
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_ok());
    }

    // ==========================================================================
    // Notes Validation Tests (T047)
    // ==========================================================================

    #[test]
    fn test_ingredient_notes_too_long() {
        let input = CreateIngredient {
            position: 1,
            quantity: None,
            unit: None,
            name: "Test".to_string(),
            notes: Some("x".repeat(501)),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_ingredient_notes_at_max() {
        let input = CreateIngredient {
            position: 1,
            quantity: None,
            unit: None,
            name: "Test".to_string(),
            notes: Some("x".repeat(500)),
        };
        assert!(input.validate().is_ok());
    }

    // ==========================================================================
    // Ingredient Serialization Tests (T047)
    // ==========================================================================

    #[test]
    fn test_ingredient_serialization_roundtrip() {
        let ingredient = Ingredient {
            id: Uuid::new_v4(),
            recipe_id: Uuid::new_v4(),
            position: 1,
            quantity: Some(dec!(2.5)),
            unit: Some("cups".to_string()),
            name: "Flour".to_string(),
            notes: Some("sifted".to_string()),
        };

        let json = serde_json::to_string(&ingredient).unwrap();
        let deserialized: Ingredient = serde_json::from_str(&json).unwrap();

        assert_eq!(ingredient.id, deserialized.id);
        assert_eq!(ingredient.name, deserialized.name);
        assert_eq!(ingredient.position, deserialized.position);
    }

    #[test]
    fn test_ingredient_serialization_with_decimal() {
        let ingredient = Ingredient {
            id: Uuid::new_v4(),
            recipe_id: Uuid::new_v4(),
            position: 1,
            quantity: Some(dec!(0.5)),
            unit: Some("tsp".to_string()),
            name: "Salt".to_string(),
            notes: None,
        };

        let json = serde_json::to_string(&ingredient).unwrap();
        assert!(json.contains("\"quantity\":"));

        let deserialized: Ingredient = serde_json::from_str(&json).unwrap();
        assert_eq!(ingredient.quantity, deserialized.quantity);
    }

    #[test]
    fn test_create_ingredient_serialization() {
        let input = CreateIngredient {
            position: 1,
            quantity: Some(dec!(100)),
            unit: Some("g".to_string()),
            name: "Butter".to_string(),
            notes: Some("softened".to_string()),
        };

        let json = serde_json::to_string(&input).unwrap();
        let deserialized: CreateIngredient = serde_json::from_str(&json).unwrap();

        assert_eq!(input.name, deserialized.name);
        assert_eq!(input.position, deserialized.position);
    }

    // ==========================================================================
    // Quantity Tests (T047)
    // ==========================================================================

    #[test]
    fn test_ingredient_quantity_zero() {
        let input = CreateIngredient {
            position: 1,
            quantity: Some(dec!(0)),
            unit: None,
            name: "Test".to_string(),
            notes: None,
        };
        // Zero quantity should be valid (no range validation on quantity)
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_ingredient_quantity_large() {
        let input = CreateIngredient {
            position: 1,
            quantity: Some(dec!(9999.99)),
            unit: Some("g".to_string()),
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_ingredient_quantity_fractional() {
        let input = CreateIngredient {
            position: 1,
            quantity: Some(dec!(0.125)),
            unit: Some("cups".to_string()),
            name: "Test".to_string(),
            notes: None,
        };
        assert!(input.validate().is_ok());
    }
}
