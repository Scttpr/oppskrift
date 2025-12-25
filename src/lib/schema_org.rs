//! Schema.org JSON-LD serialization for recipes
//!
//! Implements the Schema.org Recipe vocabulary for structured data output.
//! See: https://schema.org/Recipe

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::models::{Difficulty, Ingredient, InstructionStep, Recipe, RecipeImage, User};

/// Schema.org Recipe in JSON-LD format
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrgRecipe {
    #[serde(rename = "@context")]
    pub context: &'static str,

    #[serde(rename = "@type")]
    pub schema_type: &'static str,

    #[serde(rename = "@id")]
    pub id: String,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<SchemaOrgPerson>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_published: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_modified: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cook_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipe_yield: Option<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recipe_ingredient: Vec<String>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recipe_instructions: Vec<SchemaOrgHowToStep>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub image: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub difficulty: Option<String>,
}

/// Schema.org Person (author)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrgPerson {
    #[serde(rename = "@type")]
    pub schema_type: &'static str,

    pub name: String,

    #[serde(rename = "@id")]
    pub id: String,
}

/// Schema.org HowToStep (instruction)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaOrgHowToStep {
    #[serde(rename = "@type")]
    pub schema_type: &'static str,

    pub position: i32,

    pub text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

impl SchemaOrgRecipe {
    /// Create a Schema.org Recipe from database entities
    pub fn from_recipe(
        recipe: &Recipe,
        author: Option<&User>,
        ingredients: &[Ingredient],
        instructions: &[InstructionStep],
        images: &[RecipeImage],
    ) -> Self {
        Self {
            context: "https://schema.org",
            schema_type: "Recipe",
            id: recipe.ap_id.clone(),
            name: recipe.title.clone(),
            description: recipe.description.clone(),
            author: author.map(|a| SchemaOrgPerson {
                schema_type: "Person",
                name: a.display_name.clone(),
                id: a.ap_id.clone(),
            }),
            date_published: Some(format_iso8601(&recipe.created_at)),
            date_modified: Some(format_iso8601(&recipe.updated_at)),
            prep_time: recipe.prep_time_min.map(format_duration),
            cook_time: recipe.cook_time_min.map(format_duration),
            total_time: calculate_total_time(recipe.prep_time_min, recipe.cook_time_min),
            recipe_yield: recipe.servings.clone(),
            recipe_ingredient: ingredients
                .iter()
                .map(format_ingredient)
                .collect(),
            recipe_instructions: instructions
                .iter()
                .map(|step| SchemaOrgHowToStep {
                    schema_type: "HowToStep",
                    position: step.step_number,
                    text: step.description.clone(),
                    image: step.image_url.clone(),
                })
                .collect(),
            image: images.iter().map(|i| i.url.clone()).collect(),
            difficulty: recipe.difficulty.map(|d| match d {
                Difficulty::Easy => "Easy",
                Difficulty::Medium => "Medium",
                Difficulty::Hard => "Hard",
            }.to_string()),
        }
    }
}

/// Format a DateTime as ISO 8601
fn format_iso8601(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// Format minutes as ISO 8601 duration (PT#M)
fn format_duration(minutes: i32) -> String {
    if minutes >= 60 {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if mins > 0 {
            format!("PT{}H{}M", hours, mins)
        } else {
            format!("PT{}H", hours)
        }
    } else {
        format!("PT{}M", minutes)
    }
}

/// Calculate total time from prep and cook times
fn calculate_total_time(prep: Option<i32>, cook: Option<i32>) -> Option<String> {
    match (prep, cook) {
        (Some(p), Some(c)) => Some(format_duration(p + c)),
        (Some(p), None) => Some(format_duration(p)),
        (None, Some(c)) => Some(format_duration(c)),
        (None, None) => None,
    }
}

/// Format an ingredient for Schema.org
fn format_ingredient(ingredient: &Ingredient) -> String {
    let mut parts = Vec::new();

    if let Some(qty) = &ingredient.quantity {
        parts.push(qty.to_string());
    }

    if let Some(unit) = &ingredient.unit {
        parts.push(unit.clone());
    }

    parts.push(ingredient.name.clone());

    if let Some(notes) = &ingredient.notes {
        parts.push(format!("({})", notes));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(format_duration(30), "PT30M");
        assert_eq!(format_duration(5), "PT5M");
    }

    #[test]
    fn test_format_duration_hours() {
        assert_eq!(format_duration(60), "PT1H");
        assert_eq!(format_duration(90), "PT1H30M");
        assert_eq!(format_duration(120), "PT2H");
    }

    #[test]
    fn test_calculate_total_time() {
        assert_eq!(calculate_total_time(Some(15), Some(30)), Some("PT45M".to_string()));
        assert_eq!(calculate_total_time(Some(60), Some(60)), Some("PT2H".to_string()));
        assert_eq!(calculate_total_time(Some(30), None), Some("PT30M".to_string()));
        assert_eq!(calculate_total_time(None, None), None);
    }

    #[test]
    fn test_format_ingredient() {
        let ingredient = Ingredient {
            id: Uuid::new_v4(),
            recipe_id: Uuid::new_v4(),
            position: 1,
            quantity: Some(rust_decimal::Decimal::from(250)),
            unit: Some("g".to_string()),
            name: "flour".to_string(),
            notes: Some("all-purpose".to_string()),
        };

        let formatted = format_ingredient(&ingredient);
        assert!(formatted.contains("250"));
        assert!(formatted.contains("g"));
        assert!(formatted.contains("flour"));
        assert!(formatted.contains("(all-purpose)"));
    }
}
