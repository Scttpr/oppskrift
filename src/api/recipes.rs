use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::middleware::AuthUser;
use crate::lib::error::{AppError, AppResult};
use crate::lib::pagination::{PaginatedResponse, PaginationParams};
use crate::lib::schema_org::SchemaOrgRecipe;
use crate::models::{
    CreateIngredient, CreateInstructionStep, CreateRecipe, Ingredient, InstructionStep, Recipe,
    RecipeSummary, UpdateRecipe,
};
use crate::services::{ImageService, RecipeService, UserService};
use crate::AppState;

/// Recipe API routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_recipes).post(create_recipe))
        .route("/{id}", get(get_recipe).put(update_recipe).delete(delete_recipe))
}

/// Request body for creating a recipe with ingredients and instructions
#[derive(Debug, Deserialize)]
pub struct CreateRecipeRequest {
    #[serde(flatten)]
    pub recipe: CreateRecipe,
    #[serde(default)]
    pub ingredients: Vec<CreateIngredient>,
    #[serde(default)]
    pub instructions: Vec<CreateInstructionStep>,
}

/// Request body for updating a recipe
#[derive(Debug, Deserialize)]
pub struct UpdateRecipeRequest {
    #[serde(flatten)]
    pub recipe: UpdateRecipe,
    pub ingredients: Option<Vec<CreateIngredient>>,
    pub instructions: Option<Vec<CreateInstructionStep>>,
}

/// Full recipe response with related data
#[derive(Debug, serde::Serialize)]
pub struct RecipeResponse {
    #[serde(flatten)]
    pub recipe: Recipe,
    pub ingredients: Vec<Ingredient>,
    pub instructions: Vec<InstructionStep>,
}

/// POST /api/v1/recipes
/// Create a new recipe
async fn create_recipe(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateRecipeRequest>,
) -> AppResult<(StatusCode, Json<RecipeResponse>)> {
    // Validate ingredients and instructions
    RecipeService::validate_ingredients(&input.ingredients)?;
    RecipeService::validate_instructions(&input.instructions)?;

    // Get base URL from environment
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Create the recipe
    let recipe = RecipeService::create(&state.db, auth.id, input.recipe, &base_url).await?;

    // Add ingredients
    let ingredients = if !input.ingredients.is_empty() {
        RecipeService::add_ingredients(&state.db, recipe.id, input.ingredients).await?
    } else {
        vec![]
    };

    // Add instructions
    let instructions = if !input.instructions.is_empty() {
        RecipeService::add_instructions(&state.db, recipe.id, input.instructions).await?
    } else {
        vec![]
    };

    Ok((
        StatusCode::CREATED,
        Json(RecipeResponse {
            recipe,
            ingredients,
            instructions,
        }),
    ))
}

/// GET /api/v1/recipes/{id}
/// Get a recipe by ID (supports content negotiation for JSON-LD)
async fn get_recipe(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> AppResult<axum::response::Response> {
    use axum::response::IntoResponse;

    let recipe = RecipeService::get_by_id(&state.db, id).await?;
    let ingredients = RecipeService::get_ingredients(&state.db, id).await?;
    let instructions = RecipeService::get_instructions(&state.db, id).await?;
    let images = ImageService::get_images(&state.db, id).await?;

    // Check Accept header for JSON-LD
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    if accept.contains("application/ld+json") {
        // Get author for Schema.org
        let author = UserService::get_by_id(&state.db, recipe.author_id).await.ok();
        let schema = SchemaOrgRecipe::from_recipe(
            &recipe,
            author.as_ref(),
            &ingredients,
            &instructions,
            &images,
        );
        let json = serde_json::to_string(&schema)
            .map_err(|e| AppError::Internal(format!("Failed to serialize JSON-LD: {}", e)))?;

        Ok((
            [(header::CONTENT_TYPE, "application/ld+json")],
            json,
        )
            .into_response())
    } else {
        Ok(Json(RecipeResponse {
            recipe,
            ingredients,
            instructions,
        })
        .into_response())
    }
}

/// PUT /api/v1/recipes/{id}
/// Update a recipe
async fn update_recipe(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(input): Json<UpdateRecipeRequest>,
) -> AppResult<Json<RecipeResponse>> {
    // Check ownership
    let existing = RecipeService::get_by_id(&state.db, id).await?;
    if existing.author_id != auth.id {
        return Err(AppError::Forbidden);
    }

    // Update recipe
    let recipe = RecipeService::update(&state.db, id, input.recipe).await?;

    // Update ingredients if provided
    let ingredients = if let Some(new_ingredients) = input.ingredients {
        RecipeService::validate_ingredients(&new_ingredients)?;
        RecipeService::replace_ingredients(&state.db, id, new_ingredients).await?
    } else {
        RecipeService::get_ingredients(&state.db, id).await?
    };

    // Update instructions if provided
    let instructions = if let Some(new_instructions) = input.instructions {
        RecipeService::validate_instructions(&new_instructions)?;
        RecipeService::replace_instructions(&state.db, id, new_instructions).await?
    } else {
        RecipeService::get_instructions(&state.db, id).await?
    };

    Ok(Json(RecipeResponse {
        recipe,
        ingredients,
        instructions,
    }))
}

/// DELETE /api/v1/recipes/{id}
/// Delete a recipe
async fn delete_recipe(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    // Check ownership
    let existing = RecipeService::get_by_id(&state.db, id).await?;
    if existing.author_id != auth.id {
        return Err(AppError::Forbidden);
    }

    RecipeService::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/recipes
/// List public recipes with pagination
async fn list_recipes(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<RecipeSummary>>> {
    let recipes = RecipeService::list_public(&state.db, &params).await?;
    Ok(Json(recipes))
}
