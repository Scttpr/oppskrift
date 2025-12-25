//! Recipe seed data

use rust_decimal_macros::dec;
use sqlx::PgPool;
use uuid::Uuid;

use crate::lib::error::AppResult;
use crate::models::{CreateIngredient, CreateInstructionStep, CreateRecipe, Difficulty, Visibility};
use crate::services::RecipeService;

/// Seed sample recipes
pub async fn seed_recipes(pool: &PgPool, user_ids: &[Uuid], base_url: &str) -> AppResult<usize> {
    if user_ids.is_empty() {
        tracing::warn!("No users to assign recipes to");
        return Ok(0);
    }

    let mut count = 0;

    // Simple 3-ingredient recipe (assigned to first user)
    count += seed_simple_pasta(pool, user_ids[0], base_url).await?;

    // Medium complexity recipe (assigned to second user if available)
    let author = user_ids.get(1).copied().unwrap_or(user_ids[0]);
    count += seed_bbq_ribs(pool, author, base_url).await?;

    // Complex recipe (assigned to chef_marie if available)
    let chef = user_ids.get(2).copied().unwrap_or(user_ids[0]);
    count += seed_croissants(pool, chef, base_url).await?;

    // Stress test recipe at validation limits
    count += seed_stress_test_recipe(pool, user_ids[0], base_url).await?;

    Ok(count)
}

async fn seed_simple_pasta(pool: &PgPool, author_id: Uuid, base_url: &str) -> AppResult<usize> {
    let recipe = CreateRecipe {
        title: "Simple Garlic Pasta".to_string(),
        description: Some("A quick and delicious pasta dish that comes together in minutes.".to_string()),
        visibility: Some(Visibility::Public),
        prep_time_min: Some(5),
        cook_time_min: Some(15),
        servings: Some("2 servings".to_string()),
        difficulty: Some(Difficulty::Easy),
    };

    let created = RecipeService::create(pool, author_id, recipe, base_url).await?;

    let ingredients = vec![
        CreateIngredient {
            position: 1,
            quantity: Some(dec!(200)),
            unit: Some("g".to_string()),
            name: "spaghetti".to_string(),
            notes: None,
        },
        CreateIngredient {
            position: 2,
            quantity: Some(dec!(4)),
            unit: Some("cloves".to_string()),
            name: "garlic".to_string(),
            notes: Some("minced".to_string()),
        },
        CreateIngredient {
            position: 3,
            quantity: Some(dec!(3)),
            unit: Some("tbsp".to_string()),
            name: "olive oil".to_string(),
            notes: Some("extra virgin".to_string()),
        },
    ];

    RecipeService::add_ingredients(pool, created.id, ingredients).await?;

    let instructions = vec![
        CreateInstructionStep {
            step_number: 1,
            description: "Cook pasta according to package directions. Reserve 1 cup pasta water before draining.".to_string(),
            image_url: None,
            duration_min: Some(10),
        },
        CreateInstructionStep {
            step_number: 2,
            description: "In a large pan, heat olive oil over medium heat. Add minced garlic and cook until fragrant, about 1 minute.".to_string(),
            image_url: None,
            duration_min: Some(2),
        },
        CreateInstructionStep {
            step_number: 3,
            description: "Toss drained pasta with garlic oil. Add pasta water as needed for desired consistency. Season and serve.".to_string(),
            image_url: None,
            duration_min: Some(3),
        },
    ];

    RecipeService::add_instructions(pool, created.id, instructions).await?;
    tracing::debug!("Created recipe: Simple Garlic Pasta");
    Ok(1)
}

async fn seed_bbq_ribs(pool: &PgPool, author_id: Uuid, base_url: &str) -> AppResult<usize> {
    let recipe = CreateRecipe {
        title: "Texas-Style Smoked Ribs".to_string(),
        description: Some("Fall-off-the-bone tender ribs with a perfect smoke ring. Low and slow is the way to go!".to_string()),
        visibility: Some(Visibility::Public),
        prep_time_min: Some(30),
        cook_time_min: Some(360), // 6 hours
        servings: Some("4-6 servings".to_string()),
        difficulty: Some(Difficulty::Medium),
    };

    let created = RecipeService::create(pool, author_id, recipe, base_url).await?;

    let ingredients = vec![
        CreateIngredient { position: 1, quantity: Some(dec!(2)), unit: Some("racks".to_string()), name: "pork spare ribs".to_string(), notes: Some("about 3 lbs each".to_string()) },
        CreateIngredient { position: 2, quantity: Some(dec!(2)), unit: Some("tbsp".to_string()), name: "brown sugar".to_string(), notes: None },
        CreateIngredient { position: 3, quantity: Some(dec!(2)), unit: Some("tbsp".to_string()), name: "paprika".to_string(), notes: None },
        CreateIngredient { position: 4, quantity: Some(dec!(1)), unit: Some("tbsp".to_string()), name: "black pepper".to_string(), notes: None },
        CreateIngredient { position: 5, quantity: Some(dec!(1)), unit: Some("tbsp".to_string()), name: "salt".to_string(), notes: None },
        CreateIngredient { position: 6, quantity: Some(dec!(1)), unit: Some("tbsp".to_string()), name: "garlic powder".to_string(), notes: None },
        CreateIngredient { position: 7, quantity: Some(dec!(1)), unit: Some("tbsp".to_string()), name: "onion powder".to_string(), notes: None },
        CreateIngredient { position: 8, quantity: Some(dec!(1)), unit: Some("tsp".to_string()), name: "cayenne pepper".to_string(), notes: None },
    ];

    RecipeService::add_ingredients(pool, created.id, ingredients).await?;

    let instructions = vec![
        CreateInstructionStep { step_number: 1, description: "Remove membrane from back of ribs. Pat dry with paper towels.".to_string(), image_url: None, duration_min: Some(10) },
        CreateInstructionStep { step_number: 2, description: "Mix all dry ingredients to make the rub. Apply generously to both sides of ribs.".to_string(), image_url: None, duration_min: Some(10) },
        CreateInstructionStep { step_number: 3, description: "Let ribs sit at room temperature for 30 minutes while you prepare the smoker.".to_string(), image_url: None, duration_min: Some(30) },
        CreateInstructionStep { step_number: 4, description: "Preheat smoker to 225°F (107°C). Add oak or hickory wood chunks.".to_string(), image_url: None, duration_min: Some(20) },
        CreateInstructionStep { step_number: 5, description: "Place ribs meat-side up on the smoker. Smoke for 3 hours, spritzing with apple cider vinegar every hour.".to_string(), image_url: None, duration_min: Some(180) },
        CreateInstructionStep { step_number: 6, description: "Wrap ribs in foil with a splash of apple juice. Return to smoker for 2 hours.".to_string(), image_url: None, duration_min: Some(120) },
        CreateInstructionStep { step_number: 7, description: "Unwrap and return to smoker for final hour to firm up the bark.".to_string(), image_url: None, duration_min: Some(60) },
        CreateInstructionStep { step_number: 8, description: "Let rest 10 minutes before slicing. Serve with your favorite BBQ sauce on the side.".to_string(), image_url: None, duration_min: Some(10) },
    ];

    RecipeService::add_instructions(pool, created.id, instructions).await?;
    tracing::debug!("Created recipe: Texas-Style Smoked Ribs");
    Ok(1)
}

async fn seed_croissants(pool: &PgPool, author_id: Uuid, base_url: &str) -> AppResult<usize> {
    let recipe = CreateRecipe {
        title: "Classic French Croissants".to_string(),
        description: Some("Buttery, flaky, and absolutely worth the effort. A 3-day process that results in bakery-quality croissants.".to_string()),
        visibility: Some(Visibility::Public),
        prep_time_min: Some(180), // 3 hours active
        cook_time_min: Some(20),
        servings: Some("12 croissants".to_string()),
        difficulty: Some(Difficulty::Hard),
    };

    let created = RecipeService::create(pool, author_id, recipe, base_url).await?;

    let ingredients = vec![
        CreateIngredient { position: 1, quantity: Some(dec!(500)), unit: Some("g".to_string()), name: "bread flour".to_string(), notes: None },
        CreateIngredient { position: 2, quantity: Some(dec!(10)), unit: Some("g".to_string()), name: "instant yeast".to_string(), notes: None },
        CreateIngredient { position: 3, quantity: Some(dec!(80)), unit: Some("g".to_string()), name: "sugar".to_string(), notes: None },
        CreateIngredient { position: 4, quantity: Some(dec!(10)), unit: Some("g".to_string()), name: "salt".to_string(), notes: None },
        CreateIngredient { position: 5, quantity: Some(dec!(300)), unit: Some("ml".to_string()), name: "whole milk".to_string(), notes: Some("cold".to_string()) },
        CreateIngredient { position: 6, quantity: Some(dec!(55)), unit: Some("g".to_string()), name: "unsalted butter".to_string(), notes: Some("melted, for dough".to_string()) },
        CreateIngredient { position: 7, quantity: Some(dec!(280)), unit: Some("g".to_string()), name: "unsalted butter".to_string(), notes: Some("cold, for lamination".to_string()) },
        CreateIngredient { position: 8, quantity: Some(dec!(1)), unit: None, name: "egg".to_string(), notes: Some("for egg wash".to_string()) },
        CreateIngredient { position: 9, quantity: Some(dec!(1)), unit: Some("tbsp".to_string()), name: "milk".to_string(), notes: Some("for egg wash".to_string()) },
    ];

    RecipeService::add_ingredients(pool, created.id, ingredients).await?;

    let instructions = vec![
        CreateInstructionStep { step_number: 1, description: "Day 1: Mix flour, yeast, sugar, and salt. Add cold milk and melted butter. Knead until smooth, about 10 minutes.".to_string(), image_url: None, duration_min: Some(15) },
        CreateInstructionStep { step_number: 2, description: "Shape dough into a rectangle, wrap in plastic, and refrigerate overnight.".to_string(), image_url: None, duration_min: Some(5) },
        CreateInstructionStep { step_number: 3, description: "Day 2: Pound cold butter into a flat rectangle between parchment paper.".to_string(), image_url: None, duration_min: Some(10) },
        CreateInstructionStep { step_number: 4, description: "Roll chilled dough into a rectangle twice the size of butter. Place butter in center and fold dough over.".to_string(), image_url: None, duration_min: Some(15) },
        CreateInstructionStep { step_number: 5, description: "First turn: Roll out and fold in thirds. Refrigerate 1 hour.".to_string(), image_url: None, duration_min: Some(70) },
        CreateInstructionStep { step_number: 6, description: "Second turn: Repeat rolling and folding. Refrigerate 1 hour.".to_string(), image_url: None, duration_min: Some(70) },
        CreateInstructionStep { step_number: 7, description: "Third turn: Repeat one more time. Refrigerate overnight.".to_string(), image_url: None, duration_min: Some(15) },
        CreateInstructionStep { step_number: 8, description: "Day 3: Roll dough to 5mm thick. Cut into triangles (base 10cm, height 25cm).".to_string(), image_url: None, duration_min: Some(20) },
        CreateInstructionStep { step_number: 9, description: "Roll each triangle from base to point. Curve ends to form crescent shape.".to_string(), image_url: None, duration_min: Some(20) },
        CreateInstructionStep { step_number: 10, description: "Proof at room temperature until doubled, about 2 hours.".to_string(), image_url: None, duration_min: Some(120) },
        CreateInstructionStep { step_number: 11, description: "Brush with egg wash. Bake at 200°C (400°F) for 15-20 minutes until golden.".to_string(), image_url: None, duration_min: Some(20) },
    ];

    RecipeService::add_instructions(pool, created.id, instructions).await?;
    tracing::debug!("Created recipe: Classic French Croissants");
    Ok(1)
}

/// Create a recipe at validation limits for stress testing
async fn seed_stress_test_recipe(pool: &PgPool, author_id: Uuid, base_url: &str) -> AppResult<usize> {
    let recipe = CreateRecipe {
        title: "Stress Test Recipe (50 ingredients, 30 steps)".to_string(),
        description: Some("This recipe tests the maximum limits: 50 ingredients and 30 instruction steps.".to_string()),
        visibility: Some(Visibility::Private), // Keep private so it doesn't clutter public listings
        prep_time_min: Some(120),
        cook_time_min: Some(180),
        servings: Some("10 servings".to_string()),
        difficulty: Some(Difficulty::Hard),
    };

    let created = RecipeService::create(pool, author_id, recipe, base_url).await?;

    // Create 50 ingredients
    let ingredients: Vec<CreateIngredient> = (1..=50)
        .map(|i| CreateIngredient {
            position: i,
            quantity: Some(rust_decimal::Decimal::from(i)),
            unit: Some("g".to_string()),
            name: format!("Ingredient {}", i),
            notes: if i % 5 == 0 { Some("with notes".to_string()) } else { None },
        })
        .collect();

    RecipeService::add_ingredients(pool, created.id, ingredients).await?;

    // Create 30 instruction steps
    let instructions: Vec<CreateInstructionStep> = (1..=30)
        .map(|i| CreateInstructionStep {
            step_number: i,
            description: format!("Step {}: Perform action {} for this recipe.", i, i),
            image_url: None,
            duration_min: Some(i * 2),
        })
        .collect();

    RecipeService::add_instructions(pool, created.id, instructions).await?;
    tracing::debug!("Created stress test recipe");
    Ok(1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_recipe_module_exists() {
        // Just verify the module compiles
        assert!(true);
    }
}
