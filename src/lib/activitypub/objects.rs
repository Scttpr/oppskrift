//! ActivityPub Object implementations for Recipe and RecipeBook
//!
//! Recipes are represented as Article objects with Schema.org Recipe extensions.
//! Recipe books are represented as OrderedCollection objects.

use serde::{Deserialize, Serialize};

use super::ACTIVITYSTREAMS_CONTEXT;
use crate::models::{Ingredient, InstructionStep, Recipe, RecipeBook, RecipeImage};

/// ActivityPub Article object representing a Recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeObject {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: Vec<String>,
    #[serde(rename = "attributedTo")]
    pub attributed_to: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub content: String,
    pub published: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Vec<String>>,
    #[serde(rename = "recipeIngredient", skip_serializing_if = "Option::is_none")]
    pub recipe_ingredient: Option<Vec<String>>,
    #[serde(rename = "recipeInstructions", skip_serializing_if = "Option::is_none")]
    pub recipe_instructions: Option<Vec<RecipeInstructionObject>>,
    #[serde(rename = "prepTime", skip_serializing_if = "Option::is_none")]
    pub prep_time: Option<String>,
    #[serde(rename = "cookTime", skip_serializing_if = "Option::is_none")]
    pub cook_time: Option<String>,
    #[serde(rename = "recipeYield", skip_serializing_if = "Option::is_none")]
    pub recipe_yield: Option<String>,
}

/// HowToStep object for recipe instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeInstructionObject {
    #[serde(rename = "type")]
    pub object_type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
}

impl RecipeObject {
    /// Create a RecipeObject from a Recipe with its related data
    pub fn from_recipe(
        recipe: &Recipe,
        author_ap_id: &str,
        base_url: &str,
        ingredients: &[Ingredient],
        instructions: &[InstructionStep],
        images: &[RecipeImage],
    ) -> Self {
        // Format ingredients as strings
        let ingredient_strings: Vec<String> = ingredients
            .iter()
            .map(|i| {
                let mut s = String::new();
                if let Some(qty) = &i.quantity {
                    s.push_str(&qty.to_string());
                    s.push(' ');
                }
                if let Some(unit) = &i.unit {
                    s.push_str(unit);
                    s.push(' ');
                }
                s.push_str(&i.name);
                if let Some(notes) = &i.notes {
                    s.push_str(" (");
                    s.push_str(notes);
                    s.push(')');
                }
                s
            })
            .collect();

        // Format instructions as HowToStep objects
        let instruction_objects: Vec<RecipeInstructionObject> = instructions
            .iter()
            .map(|step| RecipeInstructionObject {
                object_type: "HowToStep".to_string(),
                text: step.description.clone(),
                position: Some(step.step_number),
            })
            .collect();

        // Collect image URLs
        let image_urls: Vec<String> = images.iter().map(|img| img.url.clone()).collect();

        // Format duration as ISO 8601
        let prep_time = recipe
            .prep_time_min
            .map(|m| format!("PT{}M", m));
        let cook_time = recipe
            .cook_time_min
            .map(|m| format!("PT{}M", m));

        // Build content from description
        let content = recipe
            .description
            .clone()
            .unwrap_or_else(|| format!("Recipe: {}", recipe.title));

        Self {
            context: serde_json::json!([
                ACTIVITYSTREAMS_CONTEXT,
                {
                    "schema": "https://schema.org/",
                    "recipeIngredient": "schema:recipeIngredient",
                    "recipeInstructions": "schema:recipeInstructions",
                    "prepTime": "schema:prepTime",
                    "cookTime": "schema:cookTime",
                    "recipeYield": "schema:recipeYield"
                }
            ]),
            id: recipe.ap_id.clone(),
            object_type: vec!["Article".to_string(), "schema:Recipe".to_string()],
            attributed_to: author_ap_id.to_string(),
            name: recipe.title.clone(),
            summary: recipe.description.clone(),
            content,
            published: recipe.created_at.to_rfc3339(),
            updated: Some(recipe.updated_at.to_rfc3339()),
            url: format!("{}/recipes/{}", base_url, recipe.id),
            image: if image_urls.is_empty() {
                None
            } else {
                Some(image_urls)
            },
            recipe_ingredient: if ingredient_strings.is_empty() {
                None
            } else {
                Some(ingredient_strings)
            },
            recipe_instructions: if instruction_objects.is_empty() {
                None
            } else {
                Some(instruction_objects)
            },
            prep_time,
            cook_time,
            recipe_yield: recipe.servings.clone(),
        }
    }
}

/// ActivityPub OrderedCollection representing a RecipeBook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeBookCollection {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: String,
    #[serde(rename = "attributedTo")]
    pub attributed_to: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(rename = "totalItems")]
    pub total_items: u64,
    #[serde(rename = "orderedItems")]
    pub ordered_items: Vec<String>,
    pub published: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

impl RecipeBookCollection {
    /// Create a RecipeBookCollection from a RecipeBook
    pub fn from_book(
        book: &RecipeBook,
        owner_ap_id: &str,
        base_url: &str,
        recipe_ap_ids: Vec<String>,
    ) -> Self {
        Self {
            context: serde_json::json!(ACTIVITYSTREAMS_CONTEXT),
            id: book.ap_id.clone(),
            object_type: "OrderedCollection".to_string(),
            attributed_to: owner_ap_id.to_string(),
            name: book.title.clone(),
            summary: book.description.clone(),
            total_items: recipe_ap_ids.len() as u64,
            ordered_items: recipe_ap_ids,
            published: book.created_at.to_rfc3339(),
            updated: Some(book.updated_at.to_rfc3339()),
            url: format!("{}/books/{}", base_url, book.id),
            image: book.cover_image_url.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_instruction_object() {
        let instruction = RecipeInstructionObject {
            object_type: "HowToStep".to_string(),
            text: "Preheat oven".to_string(),
            position: Some(1),
        };

        assert_eq!(instruction.object_type, "HowToStep");
    }
}
