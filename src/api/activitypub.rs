//! ActivityPub endpoints for federation
//!
//! Implements inbox and outbox handlers for receiving and serving activities.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::Value;
use uuid::Uuid;

use crate::lib::activitypub::{PersonActor, RecipeBookCollection, RecipeObject};
use crate::lib::pagination::PaginationParams;
use crate::services::{ActivityService, BookService, RecipeService, UserService};
use crate::AppState;

/// ActivityPub routes
pub fn routes() -> Router<AppState> {
    Router::new()
        // Actor endpoints
        .route("/users/:id", get(get_actor))
        .route("/users/:id/inbox", post(inbox))
        .route("/users/:id/outbox", get(outbox))
        .route("/users/:id/followers", get(followers))
        .route("/users/:id/following", get(following))
        // Object endpoints
        .route("/recipes/:id", get(get_recipe_object))
        .route("/books/:id", get(get_book_object))
        // Shared inbox
        .route("/inbox", post(shared_inbox))
}

/// Get actor (Person) representation
async fn get_actor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PersonActor>, StatusCode> {
    // Check Accept header for ActivityPub content type
    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !accept.contains("application/activity+json")
        && !accept.contains("application/ld+json")
    {
        return Err(StatusCode::NOT_ACCEPTABLE);
    }

    let user = UserService::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // TODO: Get actual public key from user or generate
    let public_key_pem = "-----BEGIN PUBLIC KEY-----\nPLACEHOLDER\n-----END PUBLIC KEY-----";

    let actor = PersonActor::from_user(&user, &base_url, public_key_pem);

    Ok(Json(actor))
}

/// User inbox - receive activities
async fn inbox(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _headers: HeaderMap,
    Json(activity): Json<Value>,
) -> Result<StatusCode, StatusCode> {
    // Verify the user exists
    UserService::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // TODO: Verify HTTP signature from headers
    // let signature = headers.get("signature");

    // Process the activity
    process_incoming_activity(&state, activity).await?;

    Ok(StatusCode::ACCEPTED)
}

/// Shared inbox - receive activities for any user
async fn shared_inbox(
    State(state): State<AppState>,
    _headers: HeaderMap,
    Json(activity): Json<Value>,
) -> Result<StatusCode, StatusCode> {
    // TODO: Verify HTTP signature from headers

    // Process the activity
    process_incoming_activity(&state, activity).await?;

    Ok(StatusCode::ACCEPTED)
}

/// Process an incoming activity
async fn process_incoming_activity(_state: &AppState, activity: Value) -> Result<(), StatusCode> {
    let activity_type = activity
        .get("type")
        .and_then(|t| t.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    match activity_type {
        "Follow" => {
            // Handle follow request
            // TODO: Auto-accept or queue for approval
            tracing::info!("Received Follow activity");
            Ok(())
        }
        "Undo" => {
            // Handle undo (unfollow, unlike, etc.)
            tracing::info!("Received Undo activity");
            Ok(())
        }
        "Create" | "Update" | "Delete" => {
            // Handle object mutations
            tracing::info!("Received {} activity", activity_type);
            Ok(())
        }
        "Announce" => {
            // Handle boost/share
            tracing::info!("Received Announce activity");
            Ok(())
        }
        "Like" => {
            // Handle like
            tracing::info!("Received Like activity");
            Ok(())
        }
        _ => {
            tracing::warn!("Unknown activity type: {}", activity_type);
            Ok(())
        }
    }
}

/// User outbox - list user's activities
async fn outbox(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, StatusCode> {
    let user = UserService::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Get user's activities
    let activities = ActivityService::get_user_activities(&state.db, id, &params)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let outbox = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{}/users/{}/outbox", base_url, id),
        "type": "OrderedCollection",
        "totalItems": activities.pagination.total_items,
        "first": format!("{}/users/{}/outbox?page=1", base_url, id),
    });

    Ok(Json(outbox))
}

/// User followers collection
async fn followers(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Verify user exists
    UserService::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // TODO: Get actual follower count
    let collection = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{}/users/{}/followers", base_url, id),
        "type": "OrderedCollection",
        "totalItems": 0,
    });

    Ok(Json(collection))
}

/// User following collection
async fn following(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, StatusCode> {
    // Verify user exists
    UserService::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // TODO: Get actual following count
    let collection = serde_json::json!({
        "@context": "https://www.w3.org/ns/activitystreams",
        "id": format!("{}/users/{}/following", base_url, id),
        "type": "OrderedCollection",
        "totalItems": 0,
    });

    Ok(Json(collection))
}

/// Get recipe as ActivityPub object
async fn get_recipe_object(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<RecipeObject>, StatusCode> {
    // Check Accept header
    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !accept.contains("application/activity+json")
        && !accept.contains("application/ld+json")
    {
        return Err(StatusCode::NOT_ACCEPTABLE);
    }

    let recipe = RecipeService::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get author
    let author = UserService::get_by_id(&state.db, recipe.author_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let author_ap_id = format!("{}/users/{}", base_url, author.id);

    // Get related data
    let ingredients = RecipeService::get_ingredients(&state.db, id)
        .await
        .unwrap_or_default();
    let instructions = RecipeService::get_instructions(&state.db, id)
        .await
        .unwrap_or_default();
    let images = crate::services::ImageService::get_images(&state.db, id)
        .await
        .unwrap_or_else(|_| vec![]);

    let object = RecipeObject::from_recipe(
        &recipe,
        &author_ap_id,
        &base_url,
        &ingredients,
        &instructions,
        &images,
    );

    Ok(Json(object))
}

/// Get recipe book as ActivityPub collection
async fn get_book_object(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<RecipeBookCollection>, StatusCode> {
    // Check Accept header
    let accept = headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !accept.contains("application/activity+json")
        && !accept.contains("application/ld+json")
    {
        return Err(StatusCode::NOT_ACCEPTABLE);
    }

    let book = BookService::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get owner
    let owner = UserService::get_by_id(&state.db, book.owner_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let owner_ap_id = format!("{}/users/{}", base_url, owner.id);

    // Get recipe IDs in book
    let params = PaginationParams { page: 1, page_size: 100 };
    let recipes = BookService::get_recipes_in_book(&state.db, id, &params)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let recipe_ap_ids: Vec<String> = recipes
        .data
        .iter()
        .map(|r| format!("{}/recipes/{}", base_url, r.id))
        .collect();

    let collection = RecipeBookCollection::from_book(&book, &owner_ap_id, &base_url, recipe_ap_ids);

    Ok(Json(collection))
}
