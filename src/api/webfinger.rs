//! WebFinger endpoint for ActivityPub actor discovery
//!
//! Implements RFC 7033 WebFinger for looking up actors by acct: URI.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::Deserialize;

use crate::core::activitypub::WebFingerResource;
use crate::services::UserService;
use crate::AppState;

/// WebFinger routes
pub fn routes() -> Router<AppState> {
    Router::new().route("/.well-known/webfinger", get(webfinger))
}

/// WebFinger query parameters
#[derive(Debug, Deserialize)]
pub struct WebFingerQuery {
    pub resource: String,
}

/// WebFinger endpoint
/// GET /.well-known/webfinger?resource=acct:username@domain
async fn webfinger(
    State(state): State<AppState>,
    Query(query): Query<WebFingerQuery>,
) -> Result<Json<WebFingerResource>, StatusCode> {
    // Parse the resource parameter
    let resource = &query.resource;

    // Must be an acct: URI
    let acct = resource
        .strip_prefix("acct:")
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Split username and domain
    let (username, _domain) = acct.split_once('@').ok_or(StatusCode::BAD_REQUEST)?;

    // Look up the user
    let user = UserService::get_by_username(&state.db, username)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get domain from environment or request
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let domain = base_url
        .strip_prefix("https://")
        .or_else(|| base_url.strip_prefix("http://"))
        .unwrap_or(&base_url);

    let resource = WebFingerResource::for_user(username, domain, &base_url, user.id);

    Ok(Json(resource))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routes_are_configured() {
        let _router = routes();
    }

    #[test]
    fn test_webfinger_query_deserialization() {
        // Test that WebFingerQuery can deserialize correctly
        let query: WebFingerQuery =
            serde_json::from_str(r#"{"resource": "acct:alice@example.com"}"#)
                .expect("Should deserialize");
        assert_eq!(query.resource, "acct:alice@example.com");
    }

    #[test]
    fn test_acct_uri_parsing() {
        let resource = "acct:testuser@example.com";
        let acct = resource.strip_prefix("acct:").unwrap();
        let (username, domain) = acct.split_once('@').unwrap();

        assert_eq!(username, "testuser");
        assert_eq!(domain, "example.com");
    }
}
