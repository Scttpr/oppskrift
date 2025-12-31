use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// Recipe visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "visibility_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    #[default]
    Private,
    #[sqlx(rename = "followers_only")]
    #[serde(rename = "followers_only")]
    FollowersOnly,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Public => write!(f, "Public"),
            Visibility::Private => write!(f, "Private"),
            Visibility::FollowersOnly => write!(f, "Followers Only"),
        }
    }
}

/// Recipe difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "difficulty_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl std::fmt::Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Difficulty::Easy => write!(f, "Easy"),
            Difficulty::Medium => write!(f, "Medium"),
            Difficulty::Hard => write!(f, "Hard"),
        }
    }
}

/// Recipe entity - core content type
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Recipe {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub visibility: Visibility,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub servings: Option<String>,
    pub difficulty: Option<Difficulty>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ap_id: String,
}

/// Create a new recipe
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateRecipe {
    #[validate(length(min = 1, max = 200, message = "Title must be 1-200 characters"))]
    pub title: String,
    #[validate(length(max = 2000, message = "Description must be at most 2000 characters"))]
    pub description: Option<String>,
    pub visibility: Option<Visibility>,
    #[validate(range(min = 0, max = 1440, message = "Prep time must be 0-1440 minutes"))]
    pub prep_time_min: Option<i32>,
    #[validate(range(min = 0, max = 1440, message = "Cook time must be 0-1440 minutes"))]
    pub cook_time_min: Option<i32>,
    #[validate(length(max = 100, message = "Servings must be at most 100 characters"))]
    pub servings: Option<String>,
    pub difficulty: Option<Difficulty>,
}

/// Update an existing recipe
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateRecipe {
    #[validate(length(min = 1, max = 200, message = "Title must be 1-200 characters"))]
    pub title: Option<String>,
    #[validate(length(max = 2000, message = "Description must be at most 2000 characters"))]
    pub description: Option<String>,
    pub visibility: Option<Visibility>,
    #[validate(range(min = 0, max = 1440, message = "Prep time must be 0-1440 minutes"))]
    pub prep_time_min: Option<i32>,
    #[validate(range(min = 0, max = 1440, message = "Cook time must be 0-1440 minutes"))]
    pub cook_time_min: Option<i32>,
    #[validate(length(max = 100, message = "Servings must be at most 100 characters"))]
    pub servings: Option<String>,
    pub difficulty: Option<Difficulty>,
}

/// Recipe summary for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeSummary {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub prep_time_min: Option<i32>,
    pub cook_time_min: Option<i32>,
    pub difficulty: Option<Difficulty>,
    pub created_at: DateTime<Utc>,
    pub primary_image_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    // ==========================================================================
    // Visibility Tests (T046)
    // ==========================================================================

    #[test]
    fn test_visibility_default() {
        // Default is Private for privacy-first design (ABAC spec)
        assert_eq!(Visibility::default(), Visibility::Private);
    }

    #[test]
    fn test_visibility_display() {
        assert_eq!(Visibility::Public.to_string(), "Public");
        assert_eq!(Visibility::Private.to_string(), "Private");
        assert_eq!(Visibility::FollowersOnly.to_string(), "Followers Only");
    }

    #[test]
    fn test_visibility_serialization() {
        let public = Visibility::Public;
        let json = serde_json::to_string(&public).unwrap();
        assert_eq!(json, "\"public\"");

        let private = Visibility::Private;
        let json = serde_json::to_string(&private).unwrap();
        assert_eq!(json, "\"private\"");

        let followers_only = Visibility::FollowersOnly;
        let json = serde_json::to_string(&followers_only).unwrap();
        assert_eq!(json, "\"followers_only\"");
    }

    #[test]
    fn test_visibility_deserialization() {
        let public: Visibility = serde_json::from_str("\"public\"").unwrap();
        assert_eq!(public, Visibility::Public);

        let private: Visibility = serde_json::from_str("\"private\"").unwrap();
        assert_eq!(private, Visibility::Private);

        let followers_only: Visibility = serde_json::from_str("\"followers_only\"").unwrap();
        assert_eq!(followers_only, Visibility::FollowersOnly);
    }

    // ==========================================================================
    // Difficulty Tests (T046)
    // ==========================================================================

    #[test]
    fn test_difficulty_serialization() {
        let easy = Difficulty::Easy;
        let json = serde_json::to_string(&easy).unwrap();
        assert_eq!(json, "\"easy\"");
    }

    #[test]
    fn test_difficulty_all_variants_serialization() {
        assert_eq!(
            serde_json::to_string(&Difficulty::Easy).unwrap(),
            "\"easy\""
        );
        assert_eq!(
            serde_json::to_string(&Difficulty::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&Difficulty::Hard).unwrap(),
            "\"hard\""
        );
    }

    #[test]
    fn test_difficulty_deserialization() {
        let easy: Difficulty = serde_json::from_str("\"easy\"").unwrap();
        assert_eq!(easy, Difficulty::Easy);

        let medium: Difficulty = serde_json::from_str("\"medium\"").unwrap();
        assert_eq!(medium, Difficulty::Medium);

        let hard: Difficulty = serde_json::from_str("\"hard\"").unwrap();
        assert_eq!(hard, Difficulty::Hard);
    }

    #[test]
    fn test_difficulty_display() {
        assert_eq!(Difficulty::Easy.to_string(), "Easy");
        assert_eq!(Difficulty::Medium.to_string(), "Medium");
        assert_eq!(Difficulty::Hard.to_string(), "Hard");
    }

    // ==========================================================================
    // CreateRecipe Validation Tests (T046)
    // ==========================================================================

    #[test]
    fn test_create_recipe_valid() {
        let recipe = CreateRecipe {
            title: "Chocolate Cake".to_string(),
            description: Some("A delicious cake".to_string()),
            visibility: Some(Visibility::Public),
            prep_time_min: Some(30),
            cook_time_min: Some(45),
            servings: Some("8 servings".to_string()),
            difficulty: Some(Difficulty::Medium),
        };
        assert!(recipe.validate().is_ok());
    }

    #[test]
    fn test_create_recipe_minimal() {
        let recipe = CreateRecipe {
            title: "Pasta".to_string(),
            description: None,
            visibility: None,
            prep_time_min: None,
            cook_time_min: None,
            servings: None,
            difficulty: None,
        };
        assert!(recipe.validate().is_ok());
    }

    #[test]
    fn test_create_recipe_title_empty() {
        let recipe = CreateRecipe {
            title: "".to_string(),
            description: None,
            visibility: None,
            prep_time_min: None,
            cook_time_min: None,
            servings: None,
            difficulty: None,
        };
        assert!(recipe.validate().is_err());
    }

    #[test]
    fn test_create_recipe_title_too_long() {
        let recipe = CreateRecipe {
            title: "x".repeat(201),
            description: None,
            visibility: None,
            prep_time_min: None,
            cook_time_min: None,
            servings: None,
            difficulty: None,
        };
        assert!(recipe.validate().is_err());
    }

    #[test]
    fn test_create_recipe_title_at_max() {
        let recipe = CreateRecipe {
            title: "x".repeat(200),
            description: None,
            visibility: None,
            prep_time_min: None,
            cook_time_min: None,
            servings: None,
            difficulty: None,
        };
        assert!(recipe.validate().is_ok());
    }

    #[test]
    fn test_create_recipe_description_too_long() {
        let recipe = CreateRecipe {
            title: "Test".to_string(),
            description: Some("x".repeat(2001)),
            visibility: None,
            prep_time_min: None,
            cook_time_min: None,
            servings: None,
            difficulty: None,
        };
        assert!(recipe.validate().is_err());
    }

    #[test]
    fn test_create_recipe_prep_time_negative() {
        let recipe = CreateRecipe {
            title: "Test".to_string(),
            description: None,
            visibility: None,
            prep_time_min: Some(-1),
            cook_time_min: None,
            servings: None,
            difficulty: None,
        };
        assert!(recipe.validate().is_err());
    }

    #[test]
    fn test_create_recipe_prep_time_too_large() {
        let recipe = CreateRecipe {
            title: "Test".to_string(),
            description: None,
            visibility: None,
            prep_time_min: Some(1441), // > 24 hours
            cook_time_min: None,
            servings: None,
            difficulty: None,
        };
        assert!(recipe.validate().is_err());
    }

    #[test]
    fn test_create_recipe_cook_time_negative() {
        let recipe = CreateRecipe {
            title: "Test".to_string(),
            description: None,
            visibility: None,
            prep_time_min: None,
            cook_time_min: Some(-5),
            servings: None,
            difficulty: None,
        };
        assert!(recipe.validate().is_err());
    }

    #[test]
    fn test_create_recipe_servings_too_long() {
        let recipe = CreateRecipe {
            title: "Test".to_string(),
            description: None,
            visibility: None,
            prep_time_min: None,
            cook_time_min: None,
            servings: Some("x".repeat(101)),
            difficulty: None,
        };
        assert!(recipe.validate().is_err());
    }

    // ==========================================================================
    // UpdateRecipe Validation Tests (T046)
    // ==========================================================================

    #[test]
    fn test_update_recipe_valid() {
        let update = UpdateRecipe {
            title: Some("Updated Title".to_string()),
            description: Some("New description".to_string()),
            visibility: Some(Visibility::Private),
            prep_time_min: Some(20),
            cook_time_min: Some(30),
            servings: Some("4".to_string()),
            difficulty: Some(Difficulty::Easy),
        };
        assert!(update.validate().is_ok());
    }

    #[test]
    fn test_update_recipe_all_none() {
        let update = UpdateRecipe {
            title: None,
            description: None,
            visibility: None,
            prep_time_min: None,
            cook_time_min: None,
            servings: None,
            difficulty: None,
        };
        assert!(update.validate().is_ok());
    }

    #[test]
    fn test_update_recipe_title_too_short() {
        let update = UpdateRecipe {
            title: Some("".to_string()),
            description: None,
            visibility: None,
            prep_time_min: None,
            cook_time_min: None,
            servings: None,
            difficulty: None,
        };
        assert!(update.validate().is_err());
    }

    // ==========================================================================
    // Recipe Serialization Round-Trip Tests (T046)
    // ==========================================================================

    #[test]
    fn test_recipe_serialization_roundtrip() {
        let recipe = Recipe {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "Test Recipe".to_string(),
            description: Some("A test recipe".to_string()),
            visibility: Visibility::Public,
            prep_time_min: Some(30),
            cook_time_min: Some(45),
            servings: Some("4".to_string()),
            difficulty: Some(Difficulty::Medium),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/recipes/123".to_string(),
        };

        let json = serde_json::to_string(&recipe).unwrap();
        let deserialized: Recipe = serde_json::from_str(&json).unwrap();

        assert_eq!(recipe.id, deserialized.id);
        assert_eq!(recipe.title, deserialized.title);
        assert_eq!(recipe.visibility, deserialized.visibility);
        assert_eq!(recipe.difficulty, deserialized.difficulty);
    }

    #[test]
    fn test_recipe_summary_serialization() {
        let summary = RecipeSummary {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "Summary Recipe".to_string(),
            description: None,
            prep_time_min: Some(10),
            cook_time_min: None,
            difficulty: Some(Difficulty::Easy),
            created_at: Utc::now(),
            primary_image_url: Some("https://example.com/image.jpg".to_string()),
        };

        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: RecipeSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(summary.id, deserialized.id);
        assert_eq!(summary.title, deserialized.title);
    }
}
