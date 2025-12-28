//! Sample recipe seed data

use sqlx::PgPool;
use uuid::Uuid;

use super::SeedError;

/// Recipe data structure for seeding
struct RecipeData {
    title: &'static str,
    description: &'static str,
    prep_time_min: i32,
    cook_time_min: i32,
    servings: &'static str,
    difficulty: &'static str,
    ingredients: &'static [(&'static str, &'static str, &'static str)], // (quantity, unit, name)
    instructions: &'static [&'static str],
}

const RECIPES: &[RecipeData] = &[
    // Simple recipe - Alice
    RecipeData {
        title: "Classic French Omelette",
        description: "A perfectly soft and creamy French-style omelette. Simple yet elegant.",
        prep_time_min: 5,
        cook_time_min: 3,
        servings: "1",
        difficulty: "easy",
        ingredients: &[
            ("3", "", "large eggs"),
            ("1", "tbsp", "butter"),
            ("1", "pinch", "salt"),
            ("1", "pinch", "fresh chives, chopped"),
        ],
        instructions: &[
            "Crack eggs into a bowl and beat until yolks and whites are fully combined.",
            "Heat butter in a non-stick pan over medium-high heat until foamy.",
            "Pour in eggs and stir constantly with a spatula for 30 seconds.",
            "Let set for 10 seconds, then fold and slide onto a plate.",
        ],
    },
    // Medium recipe - Alice
    RecipeData {
        title: "Tarte Tatin",
        description:
            "Classic French upside-down apple tart with caramelized apples and buttery pastry.",
        prep_time_min: 30,
        cook_time_min: 45,
        servings: "8",
        difficulty: "medium",
        ingredients: &[
            ("6", "", "Golden Delicious apples"),
            ("100", "g", "unsalted butter"),
            ("150", "g", "granulated sugar"),
            ("1", "", "sheet puff pastry"),
            ("1", "tsp", "vanilla extract"),
            ("1", "pinch", "salt"),
        ],
        instructions: &[
            "Peel, core, and quarter the apples.",
            "Melt butter in an oven-safe skillet over medium heat.",
            "Add sugar and cook until amber caramel forms, about 8 minutes.",
            "Arrange apple quarters tightly in the caramel.",
            "Cook apples for 15 minutes until slightly softened.",
            "Cover with puff pastry, tucking edges around apples.",
            "Bake at 200°C (400°F) for 25-30 minutes until golden.",
            "Let cool 5 minutes, then invert onto a serving plate.",
        ],
    },
    // BBQ recipe - Bob
    RecipeData {
        title: "Texas-Style Smoked Brisket",
        description:
            "Low and slow smoked beef brisket with a simple salt and pepper rub. The Texas way.",
        prep_time_min: 30,
        cook_time_min: 720,
        servings: "12-15",
        difficulty: "hard",
        ingredients: &[
            ("1", "", "whole packer brisket (12-14 lbs)"),
            ("1/4", "cup", "coarse black pepper"),
            ("1/4", "cup", "kosher salt"),
            ("2", "tbsp", "garlic powder"),
            ("", "", "oak or hickory wood chunks"),
        ],
        instructions: &[
            "Trim brisket, leaving 1/4 inch fat cap.",
            "Mix pepper, salt, and garlic powder. Apply liberally to all surfaces.",
            "Let brisket sit at room temperature for 1 hour.",
            "Prepare smoker to 225°F (107°C) with oak or hickory.",
            "Place brisket fat-side up. Smoke for 6-8 hours until bark forms.",
            "Wrap tightly in butcher paper when internal temp reaches 165°F.",
            "Continue smoking until internal temp reaches 203°F in the thickest part.",
            "Rest wrapped brisket for at least 1 hour before slicing against the grain.",
        ],
    },
    // Mediterranean - Chef Marie
    RecipeData {
        title: "Grilled Mediterranean Sea Bass",
        description: "Whole sea bass grilled with herbs, lemon, and olive oil. Fresh and elegant.",
        prep_time_min: 20,
        cook_time_min: 20,
        servings: "2",
        difficulty: "medium",
        ingredients: &[
            ("1", "", "whole sea bass (1.5 lbs), cleaned"),
            ("4", "tbsp", "extra virgin olive oil"),
            ("1", "", "lemon, sliced"),
            ("4", "sprigs", "fresh thyme"),
            ("4", "sprigs", "fresh rosemary"),
            ("4", "cloves", "garlic, sliced"),
            ("1", "tsp", "sea salt"),
            ("1/2", "tsp", "black pepper"),
        ],
        instructions: &[
            "Score the fish with 3 diagonal cuts on each side.",
            "Rub fish inside and out with olive oil, salt, and pepper.",
            "Stuff cavity with lemon slices, herbs, and garlic.",
            "Preheat grill to high heat (450°F).",
            "Oil grill grates well to prevent sticking.",
            "Grill fish 7-8 minutes per side until skin is crispy and flesh flakes.",
            "Rest 5 minutes, then serve with additional lemon and olive oil.",
        ],
    },
    // Dessert - Chef Marie
    RecipeData {
        title: "Chocolate Fondant",
        description:
            "Individual molten chocolate cakes with a liquid center. A restaurant classic.",
        prep_time_min: 20,
        cook_time_min: 12,
        servings: "4",
        difficulty: "medium",
        ingredients: &[
            ("200", "g", "dark chocolate (70%)"),
            ("100", "g", "unsalted butter"),
            ("3", "", "large eggs"),
            ("3", "", "egg yolks"),
            ("75", "g", "caster sugar"),
            ("50", "g", "plain flour"),
            ("1", "pinch", "salt"),
            ("", "", "butter and cocoa for ramekins"),
        ],
        instructions: &[
            "Preheat oven to 200°C (400°F). Butter and dust 4 ramekins with cocoa.",
            "Melt chocolate and butter together over a bain-marie.",
            "Whisk eggs, yolks, and sugar until thick and pale.",
            "Fold chocolate mixture into egg mixture.",
            "Sift in flour and salt, fold gently until just combined.",
            "Divide batter among ramekins.",
            "Bake 10-12 minutes until edges are set but center jiggles.",
            "Rest 1 minute, run knife around edge, invert onto plates. Serve immediately.",
        ],
    },
];

/// Seed sample recipes
///
/// Creates recipes assigned to different users.
/// Returns vector of created recipe IDs.
pub async fn seed(pool: &PgPool, user_ids: &[Uuid]) -> Result<Vec<Uuid>, SeedError> {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Assign recipes to users: alice (0,1), bob (2), chef_marie (3,4)
    let user_assignments = [0, 0, 1, 2, 2];
    let mut recipe_ids = Vec::with_capacity(RECIPES.len());

    for (idx, recipe) in RECIPES.iter().enumerate() {
        let author_id = user_ids[user_assignments[idx]];
        let ap_id = format!(
            "{}/recipes/{}",
            base_url,
            recipe.title.to_lowercase().replace(' ', "-")
        );

        // Insert recipe
        let recipe_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO recipes (author_id, title, description, prep_time_min, cook_time_min, servings, difficulty, ap_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7::difficulty_type, $8)
            RETURNING id
            "#,
        )
        .bind(author_id)
        .bind(recipe.title)
        .bind(recipe.description)
        .bind(recipe.prep_time_min)
        .bind(recipe.cook_time_min)
        .bind(recipe.servings)
        .bind(recipe.difficulty)
        .bind(&ap_id)
        .fetch_one(pool)
        .await?;

        // Insert ingredients
        for (position, (quantity, unit, name)) in recipe.ingredients.iter().enumerate() {
            let qty: Option<f64> = if quantity.is_empty() {
                None
            } else {
                quantity.parse().ok()
            };

            sqlx::query(
                r#"
                INSERT INTO ingredients (recipe_id, position, quantity, unit, name)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(recipe_id)
            .bind((position + 1) as i32)
            .bind(qty)
            .bind(if unit.is_empty() { None } else { Some(*unit) })
            .bind(name)
            .execute(pool)
            .await?;
        }

        // Insert instruction steps
        for (step_num, description) in recipe.instructions.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO instruction_steps (recipe_id, step_number, description)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(recipe_id)
            .bind((step_num + 1) as i32)
            .bind(description)
            .execute(pool)
            .await?;
        }

        recipe_ids.push(recipe_id);
    }

    Ok(recipe_ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipes_seed_data_count() {
        assert_eq!(RECIPES.len(), 5, "Should have 5 sample recipes");
    }

    #[test]
    fn test_recipes_have_ingredients() {
        for recipe in RECIPES {
            assert!(
                !recipe.ingredients.is_empty(),
                "Recipe '{}' should have ingredients",
                recipe.title
            );
        }
    }

    #[test]
    fn test_recipes_have_instructions() {
        for recipe in RECIPES {
            assert!(
                !recipe.instructions.is_empty(),
                "Recipe '{}' should have instructions",
                recipe.title
            );
        }
    }

    #[test]
    fn test_recipes_valid_difficulty() {
        let valid_difficulties = ["easy", "medium", "hard"];
        for recipe in RECIPES {
            assert!(
                valid_difficulties.contains(&recipe.difficulty),
                "Recipe '{}' has invalid difficulty: {}",
                recipe.title,
                recipe.difficulty
            );
        }
    }

    #[test]
    fn test_recipes_valid_times() {
        for recipe in RECIPES {
            assert!(
                recipe.prep_time_min >= 0,
                "Recipe '{}' has negative prep time",
                recipe.title
            );
            assert!(
                recipe.cook_time_min >= 0,
                "Recipe '{}' has negative cook time",
                recipe.title
            );
            assert!(
                recipe.cook_time_min > 0 || recipe.prep_time_min > 0,
                "Recipe '{}' should have some time",
                recipe.title
            );
        }
    }

    #[test]
    fn test_recipes_unique_titles() {
        let titles: Vec<&str> = RECIPES.iter().map(|r| r.title).collect();
        let unique: std::collections::HashSet<&str> = titles.iter().cloned().collect();
        assert_eq!(titles.len(), unique.len(), "Titles should be unique");
    }

    #[test]
    fn test_recipes_have_descriptions() {
        for recipe in RECIPES {
            assert!(
                !recipe.description.is_empty(),
                "Recipe '{}' should have description",
                recipe.title
            );
            assert!(
                recipe.description.len() >= 20,
                "Recipe '{}' description too short",
                recipe.title
            );
        }
    }
}
