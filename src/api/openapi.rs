//! OpenAPI documentation configuration
//!
//! Generates OpenAPI 3.0 specification for the API.

use axum::{routing::get, Json, Router};
use utoipa::OpenApi;

use crate::lib::error::ErrorResponse;
use crate::lib::pagination::PaginationMeta;
use crate::models::{
    LoginRequest, LoginResponse, LogoutResponse, RegisterRequest, RegisterResponse, UserProfile,
};
use crate::AppState;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Oppskrift API",
        version = "1.0.0",
        description = "A federated social platform for sharing recipes via ActivityPub",
        license(name = "AGPL-3.0", url = "https://www.gnu.org/licenses/agpl-3.0.html"),
        contact(name = "Oppskrift", url = "https://github.com/scttpr/oppskrift")
    ),
    servers(
        (url = "/api/v1", description = "API v1")
    ),
    tags(
        (name = "auth", description = "Authentication endpoints (register, login, logout, password reset)"),
        (name = "account", description = "Account management (profile, security, sessions, 2FA, deletion)"),
        (name = "recipes", description = "Recipe management endpoints"),
        (name = "books", description = "Recipe book management endpoints"),
        (name = "users", description = "User profile endpoints"),
        (name = "social", description = "Social features (follow, save, share)"),
        (name = "feed", description = "Activity feed endpoints")
    ),
    components(
        schemas(
            // Common
            ErrorResponse,
            PaginationMeta,
            // Auth
            RegisterRequest,
            RegisterResponse,
            LoginRequest,
            LoginResponse,
            LogoutResponse,
            UserProfile,
        )
    )
)]
pub struct ApiDoc;

/// OpenAPI routes
pub fn routes() -> Router<AppState> {
    Router::new().route("/openapi.json", get(openapi_json))
}

/// Serve OpenAPI JSON specification
async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
