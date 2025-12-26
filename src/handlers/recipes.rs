use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    response::Html,
    routing::get,
};
use uuid::Uuid;

use crate::AppState;
use crate::api::middleware::OptionalAuthUser;
use crate::lib::error::AppResult;
use crate::lib::pagination::{PaginationMeta, PaginationParams};
use crate::lib::schema_org::SchemaOrgRecipe;
use crate::models::{
    Ingredient, InstructionStep, Recipe, RecipeBookSummary, RecipeImage, RecipeSummary, User,
};
use crate::services::{BookService, ImageService, RecipeService, SavedRecipeService, UserService};

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
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
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
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
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
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
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
        crate::lib::error::AppError::Internal(format!("Template error: {}", e))
    })?))
}
