use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::api::middleware::rate_limit::UploadRateLimitLayer;
use crate::api::middleware::{AuthUser, OptionalAuthUser, RateLimiterState};
use crate::core::error::{AppError, AppResult};
use crate::core::pagination::{PaginatedResponse, PaginationParams};
use crate::core::schema_org::SchemaOrgRecipe;
use crate::core::storage::shared_storage;
use crate::models::{
    CreateIngredient, CreateInstructionStep, CreateRecipe, Ingredient, InstructionStep, Recipe,
    RecipeImage, RecipeSummary, Tag, UpdateRecipe,
};
use crate::models::{GrantPermissionRequest, Permission, PermissionListResponse, ResourceType};
use crate::services::{ImageService, PermissionService, RecipeService, TagService, UserService};
use crate::AppState;

/// Maximum upload size for recipe images (8 MB)
const MAX_UPLOAD_SIZE: usize = 8 * 1024 * 1024;

/// Recipe API routes (without rate limiting, for tests)
pub fn routes() -> Router<AppState> {
    // Image upload limited to MAX_UPLOAD_SIZE
    let upload_routes = Router::new()
        .route("/{id}/images", post(upload_image))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE));

    let standard_routes = Router::new()
        .route("/", get(list_recipes).post(create_recipe))
        .route("/search", get(search_recipes))
        .route(
            "/{id}",
            get(get_recipe).put(update_recipe).delete(delete_recipe),
        )
        .route("/{id}/images", get(list_images))
        .route("/{id}/images/{image_id}", delete(delete_image))
        .route("/{id}/images/{image_id}/primary", post(set_primary_image))
        .route(
            "/{id}/permissions",
            get(list_permissions).post(grant_permission),
        )
        .route("/{id}/permissions/{perm_id}", delete(revoke_permission))
        .merge(crate::api::engagement::routes());

    upload_routes.merge(standard_routes)
}

/// Recipe API routes with rate limiting applied
///
/// Rate limiting strategies:
/// - Upload: 20 uploads per 5 minutes (to prevent storage abuse)
pub fn routes_with_rate_limit(rate_limiter: RateLimiterState) -> Router<AppState> {
    // Upload endpoints with rate limiting
    let upload_routes = Router::new()
        .route("/{id}/images", post(upload_image))
        .layer(DefaultBodyLimit::max(MAX_UPLOAD_SIZE))
        .layer(UploadRateLimitLayer::new(rate_limiter));

    // Routes without special rate limiting
    let standard_routes = Router::new()
        .route("/", get(list_recipes).post(create_recipe))
        .route("/search", get(search_recipes))
        .route(
            "/{id}",
            get(get_recipe).put(update_recipe).delete(delete_recipe),
        )
        .route("/{id}/images", get(list_images))
        .route("/{id}/images/{image_id}", delete(delete_image))
        .route("/{id}/images/{image_id}/primary", post(set_primary_image))
        .route(
            "/{id}/permissions",
            get(list_permissions).post(grant_permission),
        )
        .route("/{id}/permissions/{perm_id}", delete(revoke_permission))
        .merge(crate::api::engagement::routes());

    // Merge with upload routes taking precedence for POST
    upload_routes.merge(standard_routes)
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
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Request body for updating a recipe
#[derive(Debug, Deserialize)]
pub struct UpdateRecipeRequest {
    #[serde(flatten)]
    pub recipe: UpdateRecipe,
    pub ingredients: Option<Vec<CreateIngredient>>,
    pub instructions: Option<Vec<CreateInstructionStep>>,
    pub tags: Option<Vec<String>>,
}

/// Full recipe response with related data
#[derive(Debug, serde::Serialize)]
pub struct RecipeResponse {
    #[serde(flatten)]
    pub recipe: Recipe,
    pub ingredients: Vec<Ingredient>,
    pub instructions: Vec<InstructionStep>,
    pub tags: Vec<Tag>,
}

/// POST /api/v1/recipes
/// Create a new recipe
async fn create_recipe(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateRecipeRequest>,
) -> AppResult<(StatusCode, Json<RecipeResponse>)> {
    use validator::Validate;

    // Validate recipe fields (title length, etc.), ingredients and instructions
    input
        .recipe
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    RecipeService::validate_ingredients(&input.ingredients)?;
    RecipeService::validate_instructions(&input.instructions)?;

    // Get base URL from environment
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Persist recipe, ingredients, and instructions atomically
    let mut tx = state.db.begin().await?;

    let recipe = RecipeService::create(&mut tx, auth.id, input.recipe, &base_url).await?;

    let ingredients = if !input.ingredients.is_empty() {
        RecipeService::add_ingredients(&mut tx, recipe.id, input.ingredients).await?
    } else {
        vec![]
    };

    let instructions = if !input.instructions.is_empty() {
        RecipeService::add_instructions(&mut tx, recipe.id, input.instructions).await?
    } else {
        vec![]
    };

    if !input.tags.is_empty() {
        TagService::set_recipe_tags(&mut tx, recipe.id, &input.tags).await?;
    }

    tx.commit().await?;

    let tags = TagService::get_recipe_tags(&state.db, recipe.id).await?;

    // Create ActivityPub activity for federation (after commit)
    let _ = crate::services::ActivityService::create_recipe_activity(
        &state.db, auth.id, recipe.id, &base_url,
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(RecipeResponse {
            recipe,
            ingredients,
            instructions,
            tags,
        }),
    ))
}

/// GET /api/v1/recipes/{id}
/// Get a recipe by ID (supports content negotiation for JSON-LD)
async fn get_recipe(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: OptionalAuthUser,
    headers: HeaderMap,
) -> AppResult<axum::response::Response> {
    use axum::response::IntoResponse;

    let viewer_id = auth.0.map(|u| u.id);
    let recipe = RecipeService::get_by_id_authorized(&state.db, id, viewer_id).await?;
    let ingredients = RecipeService::get_ingredients(&state.db, id).await?;
    let instructions = RecipeService::get_instructions(&state.db, id).await?;
    let images = ImageService::get_images(&state.db, id).await?;
    let tags = TagService::get_recipe_tags(&state.db, id).await?;

    // Check Accept header for JSON-LD
    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    if accept.contains("application/ld+json") {
        // Get author for Schema.org
        let author = UserService::get_by_id(&state.db, recipe.author_id)
            .await
            .ok();
        let schema = SchemaOrgRecipe::from_recipe(
            &recipe,
            author.as_ref(),
            &ingredients,
            &instructions,
            &images,
        );
        let json = serde_json::to_string(&schema)
            .map_err(|e| AppError::Internal(format!("Failed to serialize JSON-LD: {}", e)))?;

        Ok(([(header::CONTENT_TYPE, "application/ld+json")], json).into_response())
    } else {
        Ok(Json(RecipeResponse {
            recipe,
            ingredients,
            instructions,
            tags,
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
    use validator::Validate;

    // Check edit permission (returns 404 if not authorized)
    RecipeService::require_edit_permission(&state.db, id, auth.id).await?;

    // Validate recipe fields and any provided ingredients/instructions before the tx
    input
        .recipe
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if let Some(ref new_ingredients) = input.ingredients {
        RecipeService::validate_ingredients(new_ingredients)?;
    }
    if let Some(ref new_instructions) = input.instructions {
        RecipeService::validate_instructions(new_instructions)?;
    }

    // Update recipe + replace provided child collections atomically
    let mut tx = state.db.begin().await?;

    let recipe = RecipeService::update(&mut tx, id, input.recipe).await?;

    let ingredients = if let Some(new_ingredients) = input.ingredients {
        Some(RecipeService::replace_ingredients(&mut tx, id, new_ingredients).await?)
    } else {
        None
    };

    let instructions = if let Some(new_instructions) = input.instructions {
        Some(RecipeService::replace_instructions(&mut tx, id, new_instructions).await?)
    } else {
        None
    };

    if let Some(ref new_tags) = input.tags {
        TagService::set_recipe_tags(&mut tx, id, new_tags).await?;
    }

    tx.commit().await?;

    // For collections that weren't replaced, read the current state after commit
    let ingredients = match ingredients {
        Some(ingredients) => ingredients,
        None => RecipeService::get_ingredients(&state.db, id).await?,
    };
    let instructions = match instructions {
        Some(instructions) => instructions,
        None => RecipeService::get_instructions(&state.db, id).await?,
    };

    let tags = TagService::get_recipe_tags(&state.db, id).await?;

    Ok(Json(RecipeResponse {
        recipe,
        ingredients,
        instructions,
        tags,
    }))
}

/// DELETE /api/v1/recipes/{id}
/// Delete a recipe
async fn delete_recipe(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    // Check edit permission (returns 404 if not authorized)
    RecipeService::require_edit_permission(&state.db, id, auth.id).await?;

    RecipeService::delete(&state.db, id, auth.id).await?;
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

/// Query parameters for recipe search (search term + pagination)
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    /// Full-text search query
    #[serde(default)]
    pub q: String,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// GET /api/v1/recipes/search?q=...
/// Search public recipes by title and description
async fn search_recipes(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> AppResult<Json<PaginatedResponse<RecipeSummary>>> {
    let query = params.q.trim();
    if query.is_empty() {
        return Ok(Json(PaginatedResponse::new(
            vec![],
            params.pagination.page,
            params.pagination.limit(),
            0,
        )));
    }

    let recipes = RecipeService::search_public(&state.db, query, &params.pagination).await?;
    Ok(Json(recipes))
}

/// POST /api/v1/recipes/{id}/images
/// Upload an image for a recipe
async fn upload_image(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<RecipeImage>)> {
    // Check edit permission (returns 404 if not authorized)
    RecipeService::require_edit_permission(&state.db, id, auth.id).await?;

    // Create storage client
    let storage = shared_storage().await?;

    let mut image_data: Option<Vec<u8>> = None;
    let mut alt_text: Option<String> = None;
    let mut is_primary = false;
    let mut content_type: Option<String> = None;

    // Parse multipart form
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to parse multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "image" => {
                content_type = field.content_type().map(|s| s.to_string());
                image_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            AppError::BadRequest(format!("Échec de la lecture de l'image : {}", e))
                        })?
                        .to_vec(),
                );
            }
            "alt_text" => {
                let value = field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Échec de la lecture du texte alternatif : {}", e))
                })?;
                if value.len() > 1000 {
                    return Err(AppError::BadRequest(
                        "Le texte alternatif doit comporter au plus 1000 caractères".to_string(),
                    ));
                }
                alt_text = Some(value);
            }
            "is_primary" => {
                let value = field.text().await.unwrap_or_default();
                is_primary = value == "true" || value == "1";
            }
            _ => {}
        }
    }

    let data =
        image_data.ok_or_else(|| AppError::BadRequest("Aucune image fournie".to_string()))?;

    // Validate MIME type if provided
    if let Some(mime) = &content_type {
        ImageService::validate_mime_type(mime)?;
    }

    // Upload the image
    let image =
        ImageService::upload_image(&state.db, &storage, id, data, alt_text, is_primary).await?;

    Ok((StatusCode::CREATED, Json(image)))
}

/// GET /api/v1/recipes/{id}/images
/// List all images for a recipe
async fn list_images(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: OptionalAuthUser,
) -> AppResult<Json<Vec<RecipeImage>>> {
    // Verify the recipe exists and the viewer is allowed to see it
    let viewer_id = auth.0.as_ref().map(|u| u.id);
    let _ = RecipeService::get_by_id_authorized(&state.db, id, viewer_id).await?;

    let images = ImageService::get_images(&state.db, id).await?;
    Ok(Json(images))
}

/// DELETE /api/v1/recipes/{id}/images/{image_id}
/// Delete an image from a recipe
async fn delete_image(
    State(state): State<AppState>,
    Path((id, image_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    // Check edit permission (returns 404 if not authorized)
    RecipeService::require_edit_permission(&state.db, id, auth.id).await?;

    // Delete the image
    let storage = shared_storage().await?;
    ImageService::delete_image(&state.db, &storage, id, image_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/recipes/{id}/images/{image_id}/primary
/// Set an image as the primary image for a recipe
async fn set_primary_image(
    State(state): State<AppState>,
    Path((id, image_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
) -> AppResult<Json<RecipeImage>> {
    // Check edit permission (returns 404 if not authorized)
    RecipeService::require_edit_permission(&state.db, id, auth.id).await?;

    // Set as primary
    let image = ImageService::set_primary(&state.db, id, image_id).await?;
    Ok(Json(image))
}

// =============================================================================
// Permission endpoints (T040-T042)
// =============================================================================

/// POST /api/v1/recipes/{id}/permissions
/// Grant a permission on this recipe to a user, group, or instance
async fn grant_permission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(request): Json<GrantPermissionRequest>,
) -> AppResult<(StatusCode, Json<Permission>)> {
    let permission =
        PermissionService::grant_permission(&state.db, auth.id, ResourceType::Recipe, id, request)
            .await?;

    Ok((StatusCode::CREATED, Json(permission)))
}

/// DELETE /api/v1/recipes/{id}/permissions/{perm_id}
/// Revoke a permission on this recipe
async fn revoke_permission(
    State(state): State<AppState>,
    Path((id, perm_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    PermissionService::revoke_permission(&state.db, auth.id, ResourceType::Recipe, id, perm_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/recipes/{id}/permissions
/// List all permissions on this recipe (owner only)
async fn list_permissions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<Json<PermissionListResponse>> {
    let permissions =
        PermissionService::list_permissions(&state.db, auth.id, ResourceType::Recipe, id).await?;

    Ok(Json(PermissionListResponse {
        permissions,
        resource_type: "recipe".to_string(),
        resource_id: id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_are_configured() {
        let _router = routes();
    }
}
