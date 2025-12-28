use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Instruction step entity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InstructionStep {
    pub id: Uuid,
    pub recipe_id: Uuid,
    pub step_number: i32,
    pub description: String,
    pub image_url: Option<String>,
    pub duration_min: Option<i32>,
}

/// Create a new instruction step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInstructionStep {
    pub step_number: i32,
    pub description: String,
    pub image_url: Option<String>,
    pub duration_min: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // CreateInstructionStep Basic Tests (T048)
    // ==========================================================================

    #[test]
    fn test_create_instruction_step() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "Preheat oven to 180°C".to_string(),
            image_url: None,
            duration_min: Some(5),
        };

        assert_eq!(step.step_number, 1);
        assert_eq!(step.duration_min, Some(5));
    }

    #[test]
    fn test_create_instruction_step_minimal() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "Mix ingredients".to_string(),
            image_url: None,
            duration_min: None,
        };

        assert_eq!(step.step_number, 1);
        assert!(step.image_url.is_none());
        assert!(step.duration_min.is_none());
    }

    #[test]
    fn test_create_instruction_step_full() {
        let step = CreateInstructionStep {
            step_number: 5,
            description: "Bake until golden brown".to_string(),
            image_url: Some("https://example.com/step5.jpg".to_string()),
            duration_min: Some(30),
        };

        assert_eq!(step.step_number, 5);
        assert_eq!(
            step.image_url,
            Some("https://example.com/step5.jpg".to_string())
        );
        assert_eq!(step.duration_min, Some(30));
    }

    // ==========================================================================
    // Step Number Tests (T048)
    // ==========================================================================

    #[test]
    fn test_instruction_step_number_one() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "First step".to_string(),
            image_url: None,
            duration_min: None,
        };
        assert_eq!(step.step_number, 1);
    }

    #[test]
    fn test_instruction_step_number_large() {
        let step = CreateInstructionStep {
            step_number: 30,
            description: "Last step".to_string(),
            image_url: None,
            duration_min: None,
        };
        assert_eq!(step.step_number, 30);
    }

    #[test]
    fn test_instruction_step_number_zero() {
        // Zero is technically allowed at struct level (no validation attribute)
        let step = CreateInstructionStep {
            step_number: 0,
            description: "Zero step".to_string(),
            image_url: None,
            duration_min: None,
        };
        assert_eq!(step.step_number, 0);
    }

    // ==========================================================================
    // Duration Tests (T048)
    // ==========================================================================

    #[test]
    fn test_instruction_step_duration_none() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "Quick step".to_string(),
            image_url: None,
            duration_min: None,
        };
        assert!(step.duration_min.is_none());
    }

    #[test]
    fn test_instruction_step_duration_short() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "Mix quickly".to_string(),
            image_url: None,
            duration_min: Some(1),
        };
        assert_eq!(step.duration_min, Some(1));
    }

    #[test]
    fn test_instruction_step_duration_long() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "Slow roast".to_string(),
            image_url: None,
            duration_min: Some(180),
        };
        assert_eq!(step.duration_min, Some(180));
    }

    // ==========================================================================
    // InstructionStep Serialization Tests (T048)
    // ==========================================================================

    #[test]
    fn test_instruction_step_serialization_roundtrip() {
        let step = InstructionStep {
            id: Uuid::new_v4(),
            recipe_id: Uuid::new_v4(),
            step_number: 1,
            description: "Preheat oven to 180°C".to_string(),
            image_url: Some("https://example.com/step.jpg".to_string()),
            duration_min: Some(5),
        };

        let json = serde_json::to_string(&step).unwrap();
        let deserialized: InstructionStep = serde_json::from_str(&json).unwrap();

        assert_eq!(step.id, deserialized.id);
        assert_eq!(step.step_number, deserialized.step_number);
        assert_eq!(step.description, deserialized.description);
        assert_eq!(step.image_url, deserialized.image_url);
        assert_eq!(step.duration_min, deserialized.duration_min);
    }

    #[test]
    fn test_instruction_step_serialization_minimal() {
        let step = InstructionStep {
            id: Uuid::new_v4(),
            recipe_id: Uuid::new_v4(),
            step_number: 1,
            description: "Simple step".to_string(),
            image_url: None,
            duration_min: None,
        };

        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("\"step_number\":1"));
        assert!(json.contains("\"description\":\"Simple step\""));

        let deserialized: InstructionStep = serde_json::from_str(&json).unwrap();
        assert_eq!(step.description, deserialized.description);
    }

    #[test]
    fn test_create_instruction_step_serialization() {
        let step = CreateInstructionStep {
            step_number: 2,
            description: "Add eggs".to_string(),
            image_url: None,
            duration_min: Some(2),
        };

        let json = serde_json::to_string(&step).unwrap();
        let deserialized: CreateInstructionStep = serde_json::from_str(&json).unwrap();

        assert_eq!(step.step_number, deserialized.step_number);
        assert_eq!(step.description, deserialized.description);
    }

    // ==========================================================================
    // Description Tests (T048)
    // ==========================================================================

    #[test]
    fn test_instruction_step_description_short() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "Mix".to_string(),
            image_url: None,
            duration_min: None,
        };
        assert_eq!(step.description, "Mix");
    }

    #[test]
    fn test_instruction_step_description_long() {
        let long_description = "This is a very detailed instruction step that explains exactly what needs to be done, including specific techniques and tips for best results. Make sure to follow each part carefully for optimal outcome.".to_string();
        let step = CreateInstructionStep {
            step_number: 1,
            description: long_description.clone(),
            image_url: None,
            duration_min: None,
        };
        assert_eq!(step.description, long_description);
    }

    #[test]
    fn test_instruction_step_description_with_unicode() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "Préchauffer le four à 180°C 🔥".to_string(),
            image_url: None,
            duration_min: None,
        };
        assert!(step.description.contains("°C"));
        assert!(step.description.contains("🔥"));
    }

    // ==========================================================================
    // Image URL Tests (T048)
    // ==========================================================================

    #[test]
    fn test_instruction_step_image_url_valid() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "See image".to_string(),
            image_url: Some("https://cdn.example.com/recipes/123/step1.jpg".to_string()),
            duration_min: None,
        };
        assert!(step.image_url.is_some());
        assert!(step.image_url.unwrap().starts_with("https://"));
    }

    #[test]
    fn test_instruction_step_image_url_none() {
        let step = CreateInstructionStep {
            step_number: 1,
            description: "No image".to_string(),
            image_url: None,
            duration_min: None,
        };
        assert!(step.image_url.is_none());
    }
}
