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
}
