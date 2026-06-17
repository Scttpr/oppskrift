use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use uuid::Uuid;

use crate::api::middleware::{AuthUser, OptionalAuthUser};
use crate::core::error::{AppError, AppResult};
use crate::core::pagination::{PaginatedResponse, PaginationParams};
use crate::core::storage::shared_storage;
use crate::models::{
    AddContributionRequest, AddRecipeToBook, BookContribution, BookRecipeEntry, CreateRecipeBook,
    GrantPermissionRequest, Permission, PermissionListResponse, RecipeBook, RecipeBookSummary,
    RecipeSummary, ResourceType, UpdateRecipeBook,
};
use crate::services::{BookContributionService, BookService, PermissionService};
use crate::AppState;

/// Book API routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_books).post(create_book))
        .route("/{id}", get(get_book).put(update_book).delete(delete_book))
        .route(
            "/{id}/recipes",
            get(get_book_recipes).post(add_recipe_to_book),
        )
        .route("/{id}/recipes/{recipe_id}", delete(remove_recipe_from_book))
        .route(
            "/{id}/contributions",
            get(list_contributions).post(add_contribution),
        )
        .route(
            "/{id}/contributions/{recipe_id}",
            delete(remove_contribution),
        )
        .route(
            "/{id}/permissions",
            get(list_permissions).post(grant_permission),
        )
        .route("/{id}/permissions/{perm_id}", delete(revoke_permission))
}

/// Book response with recipe count
#[derive(Debug, serde::Serialize)]
pub struct BookResponse {
    #[serde(flatten)]
    pub book: RecipeBook,
    pub recipe_count: i64,
}

/// POST /api/v1/books
/// Create a new recipe book (supports multipart for cover image)
async fn create_book(
    State(state): State<AppState>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<RecipeBook>)> {
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut visibility: Option<String> = None;
    let mut cover_image_url: Option<String> = None;

    // Parse multipart form
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to parse multipart: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "title" => {
                title =
                    Some(field.text().await.map_err(|e| {
                        AppError::BadRequest(format!("Failed to read title: {}", e))
                    })?);
            }
            "description" => {
                description = Some(field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read description: {}", e))
                })?);
            }
            "visibility" => {
                visibility = Some(field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read visibility: {}", e))
                })?);
            }
            "cover_image" => {
                let content_type = field.content_type().map(|s| s.to_string());
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(format!("Failed to read image: {}", e)))?
                    .to_vec();

                if !data.is_empty() {
                    // Validate and upload cover image
                    if let Some(mime) = &content_type {
                        crate::services::ImageService::validate_mime_type(mime)?;
                    }

                    let storage = shared_storage().await?;
                    let book_id = Uuid::new_v4();
                    let key = format!("books/{}/cover.webp", book_id);

                    let (processed, content_type) =
                        crate::services::ImageService::process_image(&data).await?;
                    cover_image_url = Some(storage.upload(&key, processed, &content_type).await?);
                }
            }
            _ => {}
        }
    }

    let title = title.ok_or_else(|| AppError::BadRequest("Title is required".to_string()))?;

    let visibility_enum = match visibility.as_deref() {
        Some("private") => Some(crate::models::Visibility::Private),
        _ => Some(crate::models::Visibility::Public),
    };

    let input = CreateRecipeBook {
        title,
        description,
        cover_image_url,
        visibility: visibility_enum,
    };

    let book = BookService::create(&state.db, auth.id, input, &base_url).await?;

    // Create ActivityPub activity for federation
    let _ = crate::services::ActivityService::create_book_activity(
        &state.db, auth.id, book.id, &base_url,
    )
    .await;

    Ok((StatusCode::CREATED, Json(book)))
}

/// GET /api/v1/books/{id}
/// Get a recipe book by ID (respects visibility)
async fn get_book(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: OptionalAuthUser,
) -> AppResult<Json<BookResponse>> {
    let viewer_id = auth.0.map(|u| u.id);
    let book = BookService::get_by_id_authorized(&state.db, id, viewer_id).await?;

    // Get recipe count
    let recipe_count = BookService::get_recipe_count(&state.db, id).await?;

    Ok(Json(BookResponse { book, recipe_count }))
}

/// PUT /api/v1/books/{id}
/// Update a recipe book
async fn update_book(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(input): Json<UpdateRecipeBook>,
) -> AppResult<Json<RecipeBook>> {
    // Check edit permission (returns 404 if not authorized)
    BookService::require_edit_permission(&state.db, id, auth.id).await?;

    let book = BookService::update(&state.db, id, input).await?;
    Ok(Json(book))
}

/// DELETE /api/v1/books/{id}
/// Delete a recipe book
async fn delete_book(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    // Check edit permission (returns 404 if not authorized)
    BookService::require_edit_permission(&state.db, id, auth.id).await?;

    BookService::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/books
/// List public books with pagination
async fn list_books(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<RecipeBookSummary>>> {
    let books = BookService::list_public(&state.db, &params).await?;
    Ok(Json(books))
}

/// GET /api/v1/books/{id}/recipes
/// Get recipes in a book (respects book visibility and recipe visibility)
async fn get_book_recipes(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: OptionalAuthUser,
    Query(params): Query<PaginationParams>,
) -> AppResult<Json<PaginatedResponse<RecipeSummary>>> {
    let viewer_id = auth.0.map(|u| u.id);

    // First check if viewer can access the book
    let book = BookService::get_by_id_authorized(&state.db, id, viewer_id).await?;

    // If viewer is the owner, show all recipes; otherwise show only public ones
    if viewer_id == Some(book.owner_id) {
        let recipes = BookService::get_recipes_in_book(&state.db, id, &params).await?;
        Ok(Json(recipes))
    } else {
        let recipes = BookService::get_public_recipes_in_book(&state.db, id, &params).await?;
        Ok(Json(recipes))
    }
}

/// POST /api/v1/books/{id}/recipes
/// Add a recipe to a book
async fn add_recipe_to_book(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(input): Json<AddRecipeToBook>,
) -> AppResult<(StatusCode, Json<BookRecipeEntry>)> {
    // Check edit permission (returns 404 if not authorized)
    BookService::require_edit_permission(&state.db, id, auth.id).await?;

    let entry = BookService::add_recipe(&state.db, id, input).await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

/// DELETE /api/v1/books/{id}/recipes/{recipe_id}
/// Remove a recipe from a book
async fn remove_recipe_from_book(
    State(state): State<AppState>,
    Path((id, recipe_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    // Check edit permission (returns 404 if not authorized)
    BookService::require_edit_permission(&state.db, id, auth.id).await?;

    BookService::remove_recipe(&state.db, id, recipe_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// =============================================================================
// Permission endpoints (T043-T045)
// =============================================================================

/// POST /api/v1/books/{id}/permissions
/// Grant a permission on this book to a user, group, or instance
async fn grant_permission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(request): Json<GrantPermissionRequest>,
) -> AppResult<(StatusCode, Json<Permission>)> {
    let permission =
        PermissionService::grant_permission(&state.db, auth.id, ResourceType::Book, id, request)
            .await?;

    Ok((StatusCode::CREATED, Json(permission)))
}

/// DELETE /api/v1/books/{id}/permissions/{perm_id}
/// Revoke a permission on this book
async fn revoke_permission(
    State(state): State<AppState>,
    Path((id, perm_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    PermissionService::revoke_permission(&state.db, auth.id, ResourceType::Book, id, perm_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/books/{id}/permissions
/// List all permissions on this book (owner only)
async fn list_permissions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> AppResult<Json<PermissionListResponse>> {
    let permissions =
        PermissionService::list_permissions(&state.db, auth.id, ResourceType::Book, id).await?;

    Ok(Json(PermissionListResponse {
        permissions,
        resource_type: "book".to_string(),
        resource_id: id,
    }))
}

// =============================================================================
// Contribution endpoints (T079-T080)
// =============================================================================

/// POST /api/v1/books/{id}/contributions
/// Add a recipe as a contribution to a book (contributor permission required)
async fn add_contribution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(request): Json<AddContributionRequest>,
) -> AppResult<(StatusCode, Json<BookContribution>)> {
    let contribution =
        BookContributionService::add_contribution(&state.db, id, auth.id, request).await?;

    Ok((StatusCode::CREATED, Json(contribution)))
}

/// DELETE /api/v1/books/{id}/contributions/{recipe_id}
/// Remove a contribution (contributor can remove own, owner can remove any)
async fn remove_contribution(
    State(state): State<AppState>,
    Path((id, recipe_id)): Path<(Uuid, Uuid)>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    BookContributionService::remove_contribution(&state.db, id, recipe_id, auth.id).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/books/{id}/contributions
/// List all contributions for a book
async fn list_contributions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: OptionalAuthUser,
) -> AppResult<Json<Vec<crate::models::BookContributionWithDisplay>>> {
    let viewer_id = auth.0.map(|u| u.id);

    // Verify the user can access the book
    let _ = BookService::get_by_id_authorized(&state.db, id, viewer_id).await?;

    let contributions = BookContributionService::get_contributions(&state.db, id).await?;
    Ok(Json(contributions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_are_configured() {
        let _router = routes();
    }
}
