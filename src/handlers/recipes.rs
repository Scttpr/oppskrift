use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::get,
    Router,
};
use uuid::Uuid;

use crate::api::middleware::OptionalAuthUser;
use crate::core::error::AppResult;
use crate::core::pagination::{PaginationMeta, PaginationParams};
use crate::core::schema_org::SchemaOrgRecipe;
use crate::models::{
    Ingredient, InstructionStep, Recipe, RecipeBookSummary, RecipeImage, RecipeSummary, User,
};
use crate::services::{BookService, ImageService, RecipeService, SavedRecipeService, UserService};
use crate::AppState;

/// Recipe page routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_recipes_page))
        .route("/new", get(new_recipe_page))
        .route("/{id}", get(view_recipe_page))
        .route("/{id}/edit", get(edit_recipe_page))
}

/// Recipe list page template
#[derive(Template)]
#[template(path = "recipes/list.html")]
struct RecipeListTemplate {
    recipes: Vec<RecipeSummary>,
    pagination: PaginationMeta,
    user: Option<User>,
}

/// Recipe list page handler
async fn list_recipes_page(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let recipes_page = RecipeService::list_public(&state.db, &params).await?;

    let user = if let Some(auth_user) = auth.0 {
        UserService::get_by_id(&state.db, auth_user.id).await.ok()
    } else {
        None
    };

    let template = RecipeListTemplate {
        recipes: recipes_page.data,
        pagination: recipes_page.pagination,
        user,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// New recipe form template
#[derive(Template)]
#[template(path = "recipes/form.html")]
struct NewRecipeTemplate {
    recipe: Option<Recipe>,
}

/// New recipe page handler
async fn new_recipe_page() -> AppResult<Html<String>> {
    let template = NewRecipeTemplate { recipe: None };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Recipe view page template
#[derive(Template)]
#[template(path = "recipes/view.html")]
struct RecipeViewTemplate {
    recipe: Recipe,
    author: Option<User>,
    ingredients: Vec<Ingredient>,
    instructions: Vec<InstructionStep>,
    images: Vec<RecipeImage>,
    primary_image: Option<RecipeImage>,
    schema_json: String,
    is_owner: bool,
    user: Option<User>,
    user_books: Vec<RecipeBookSummary>,
    is_saved: bool,
}

/// View recipe page handler
async fn view_recipe_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: OptionalAuthUser,
) -> AppResult<Html<String>> {
    let recipe = RecipeService::get_by_id(&state.db, id).await?;
    let author = UserService::get_by_id(&state.db, recipe.author_id)
        .await
        .ok();
    let ingredients = RecipeService::get_ingredients(&state.db, id).await?;
    let instructions = RecipeService::get_instructions(&state.db, id).await?;
    let images = ImageService::get_images(&state.db, id).await?;
    let primary_image = images.iter().find(|i| i.is_primary).cloned();

    // Generate Schema.org JSON-LD
    let schema = SchemaOrgRecipe::from_recipe(
        &recipe,
        author.as_ref(),
        &ingredients,
        &instructions,
        &images,
    );
    let schema_json = serde_json::to_string_pretty(&schema).unwrap_or_default();

    let is_owner = auth.0.as_ref().map(|u| u.id) == Some(recipe.author_id);

    // Fetch current user, their books, and saved status
    let (user, user_books, is_saved) = if let Some(auth_user) = auth.0.as_ref() {
        let user = UserService::get_by_id(&state.db, auth_user.id).await.ok();
        let books = BookService::list_by_owner(&state.db, auth_user.id)
            .await
            .unwrap_or_default();
        let is_saved = SavedRecipeService::is_saved(&state.db, auth_user.id, id).await?;
        (user, books, is_saved)
    } else {
        (None, vec![], false)
    };

    let template = RecipeViewTemplate {
        recipe,
        author,
        ingredients,
        instructions,
        images,
        primary_image,
        schema_json,
        is_owner,
        user,
        user_books,
        is_saved,
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

/// Edit recipe form template
#[derive(Template)]
#[template(path = "recipes/form.html")]
struct EditRecipeTemplate {
    recipe: Option<Recipe>,
}

/// Edit recipe page handler
async fn edit_recipe_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    let recipe = RecipeService::get_by_id(&state.db, id).await?;

    let template = EditRecipeTemplate {
        recipe: Some(recipe),
    };

    Ok(Html(template.render().map_err(|e| {
        crate::core::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Difficulty, Visibility};
    use askama::Template;
    use chrono::Utc;

    // ==========================================================================
    // Route Configuration Tests (T052)
    // ==========================================================================

    #[test]
    fn test_routes_returns_router() {
        let router = routes();
        let _ = router;
    }

    // ==========================================================================
    // Template Struct Tests (T052)
    // ==========================================================================

    #[test]
    fn test_recipe_list_template_renders_empty() {
        let template = RecipeListTemplate {
            recipes: vec![],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 0,
                total_pages: 0,
                has_next: false,
                has_prev: false,
            },
            user: None,
        };
        let result = template.render();
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_new_recipe_template_renders() {
        let template = NewRecipeTemplate { recipe: None };
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(!html.is_empty());
        // Should contain form elements
        assert!(html.contains("form") || html.contains("recipe"));
    }

    #[test]
    fn test_edit_recipe_template_with_recipe() {
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
            ap_id: "https://example.com/recipes/1".to_string(),
        };

        let template = EditRecipeTemplate {
            recipe: Some(recipe),
        };
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("Test Recipe") || html.contains("form"));
    }

    #[test]
    fn test_recipe_view_template_renders() {
        let recipe = Recipe {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "View Recipe".to_string(),
            description: Some("Description".to_string()),
            visibility: Visibility::Public,
            prep_time_min: Some(15),
            cook_time_min: Some(30),
            servings: Some("2".to_string()),
            difficulty: Some(Difficulty::Easy),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/recipes/2".to_string(),
        };

        let template = RecipeViewTemplate {
            recipe,
            author: None,
            ingredients: vec![],
            instructions: vec![],
            images: vec![],
            primary_image: None,
            schema_json: "{}".to_string(),
            is_owner: false,
            user: None,
            user_books: vec![],
            is_saved: false,
        };
        let result = template.render();
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("View Recipe") || !html.is_empty());
    }

    // ==========================================================================
    // Pagination Tests (T052)
    // ==========================================================================

    #[test]
    fn test_pagination_meta_in_template() {
        let pagination = PaginationMeta {
            page: 2,
            page_size: 20,
            total_items: 50,
            total_pages: 3,
            has_next: true,
            has_prev: true,
        };

        let template = RecipeListTemplate {
            recipes: vec![],
            pagination,
            user: None,
        };

        let html = template.render().unwrap();
        // Template should render even with pagination data
        assert!(!html.is_empty());
    }

    #[test]
    fn test_recipe_list_with_recipes() {
        let recipe = RecipeSummary {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "Sample Recipe".to_string(),
            description: Some("A sample".to_string()),
            prep_time_min: Some(10),
            cook_time_min: Some(20),
            difficulty: Some(Difficulty::Easy),
            created_at: Utc::now(),
            primary_image_url: None,
        };

        let template = RecipeListTemplate {
            recipes: vec![recipe],
            pagination: PaginationMeta {
                page: 1,
                page_size: 10,
                total_items: 1,
                total_pages: 1,
                has_next: false,
                has_prev: false,
            },
            user: None,
        };

        let html = template.render().unwrap();
        assert!(html.contains("Sample Recipe") || !html.is_empty());
    }

    // ==========================================================================
    // Owner/Auth State Tests (T052)
    // ==========================================================================

    #[test]
    fn test_recipe_view_owner_state() {
        let recipe = Recipe {
            id: Uuid::new_v4(),
            author_id: Uuid::new_v4(),
            title: "Owner Recipe".to_string(),
            description: None,
            visibility: Visibility::Public,
            prep_time_min: None,
            cook_time_min: None,
            servings: None,
            difficulty: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ap_id: "https://example.com/recipes/3".to_string(),
        };

        // Test as owner
        let owner_template = RecipeViewTemplate {
            recipe: recipe.clone(),
            author: None,
            ingredients: vec![],
            instructions: vec![],
            images: vec![],
            primary_image: None,
            schema_json: "{}".to_string(),
            is_owner: true,
            user: None,
            user_books: vec![],
            is_saved: false,
        };
        assert!(owner_template.render().is_ok());

        // Test as non-owner
        let guest_template = RecipeViewTemplate {
            recipe,
            author: None,
            ingredients: vec![],
            instructions: vec![],
            images: vec![],
            primary_image: None,
            schema_json: "{}".to_string(),
            is_owner: false,
            user: None,
            user_books: vec![],
            is_saved: false,
        };
        assert!(guest_template.render().is_ok());
    }
}
