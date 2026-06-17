use axum::{
    extract::{Multipart, Path, Query, State},
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
use crate::core::storage::StorageClient;
use crate::models::{
    CreateIngredient, CreateInstructionStep, CreateRecipe, Ingredient, InstructionStep, Recipe,
    RecipeImage, RecipeSummary, UpdateRecipe,
};
use crate::models::{GrantPermissionRequest, Permission, PermissionListResponse, ResourceType};
use crate::services::{ImageService, PermissionService, RecipeService, UserService};
use crate::AppState;

/// Recipe API routes (without rate limiting, for tests)
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_recipes).post(create_recipe))
        .route(
            "/{id}",
            get(get_recipe).put(update_recipe).delete(delete_recipe),
        )
        .route("/{id}/images", get(list_images).post(upload_image))
        .route("/{id}/images/{image_id}", delete(delete_image))
        .route("/{id}/images/{image_id}/primary", post(set_primary_image))
        .route(
            "/{id}/permissions",
            get(list_permissions).post(grant_permission),
        )
        .route("/{id}/permissions/{perm_id}", delete(revoke_permission))
}

/// Recipe API routes with rate limiting applied
///
/// Rate limiting strategies:
/// - Upload: 20 uploads per 5 minutes (to prevent storage abuse)
pub fn routes_with_rate_limit(rate_limiter: RateLimiterState) -> Router<AppState> {
    // Upload endpoints with rate limiting
    let upload_routes = Router::new()
        .route("/{id}/images", post(upload_image))
        .layer(UploadRateLimitLayer::new(rate_limiter));

    // Routes without special rate limiting
    let standard_routes = Router::new()
        .route("/", get(list_recipes).post(create_recipe))
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
        .route("/{id}/permissions/{perm_id}", delete(revoke_permission));

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
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

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

    // Create ActivityPub activity for federation
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
    // Check edit permission (returns 404 if not authorized)
    RecipeService::require_edit_permission(&state.db, id, auth.id).await?;

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
    let storage = StorageClient::from_env().await?;

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
                        .map_err(|e| AppError::BadRequest(format!("Failed to read image: {}", e)))?
                        .to_vec(),
                );
            }
            "alt_text" => {
                alt_text = Some(field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read alt_text: {}", e))
                })?);
            }
            "is_primary" => {
                let value = field.text().await.unwrap_or_default();
                is_primary = value == "true" || value == "1";
            }
            _ => {}
        }
    }

    let data = image_data.ok_or_else(|| AppError::BadRequest("No image provided".to_string()))?;

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
    let storage = StorageClient::from_env().await?;
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
