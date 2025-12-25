//! oEmbed endpoint for recipe embedding
//!
//! Implements oEmbed 1.0 specification for embedding recipes.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::{ImageService, RecipeService, UserService};
use crate::AppState;

/// oEmbed routes
pub fn routes() -> Router<AppState> {
    Router::new().route("/oembed", get(oembed))
}

/// oEmbed query parameters
#[derive(Debug, Deserialize)]
pub struct OEmbedQuery {
    pub url: String,
    #[serde(default = "default_max_width")]
    pub maxwidth: u32,
    #[serde(default = "default_max_height")]
    pub maxheight: u32,
    #[serde(default)]
    pub format: Option<String>,
}

fn default_max_width() -> u32 {
    640
}

fn default_max_height() -> u32 {
    480
}

/// oEmbed response
#[derive(Debug, Serialize)]
pub struct OEmbedResponse {
    #[serde(rename = "type")]
    pub oembed_type: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_age: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_height: Option<u32>,
    // Rich type specific
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}

/// oEmbed endpoint
async fn oembed(
    State(state): State<AppState>,
    Query(query): Query<OEmbedQuery>,
) -> Result<Json<OEmbedResponse>, StatusCode> {
    // Only support JSON format
    if let Some(ref format) = query.format {
        if format != "json" {
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
    }

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Parse the URL to extract recipe ID
    let recipe_id = parse_recipe_url(&query.url, &base_url)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get recipe
    let recipe = RecipeService::get_by_id(&state.db, recipe_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get author
    let author = UserService::get_by_id(&state.db, recipe.author_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get primary image if available
    let primary_image = ImageService::get_primary_image(&state.db, recipe_id)
        .await
        .ok()
        .flatten();

    let author_name = &author.display_name;

    // Calculate embed dimensions respecting max constraints
    let width = query.maxwidth.min(640);
    let height = query.maxheight.min(400);

    // Build embed HTML
    let html = format!(
        r#"<div style="border: 1px solid #ddd; border-radius: 8px; padding: 16px; max-width: {}px; font-family: system-ui, -apple-system, sans-serif;">
  <h3 style="margin: 0 0 8px 0;"><a href="{}/recipes/{}" style="color: #2563eb; text-decoration: none;">{}</a></h3>
  <p style="margin: 0 0 8px 0; color: #666;">{}</p>
  <p style="margin: 0; font-size: 0.875rem; color: #888;">By <a href="{}/users/{}" style="color: #2563eb; text-decoration: none;">{}</a> on Oppskrift</p>
</div>"#,
        width,
        base_url,
        recipe.id,
        html_escape(&recipe.title),
        html_escape(&recipe.description.clone().unwrap_or_default()),
        base_url,
        author.id,
        html_escape(author_name)
    );

    let response = OEmbedResponse {
        oembed_type: "rich".to_string(),
        version: "1.0".to_string(),
        title: Some(recipe.title),
        author_name: Some(author_name.clone()),
        author_url: Some(format!("{}/users/{}", base_url, author.id)),
        provider_name: Some("Oppskrift".to_string()),
        provider_url: Some(base_url.clone()),
        cache_age: Some(3600),
        thumbnail_url: primary_image.as_ref().map(|img| img.url.clone()),
        thumbnail_width: primary_image.as_ref().map(|_| 300),
        thumbnail_height: primary_image.as_ref().map(|_| 200),
        html: Some(html),
        width: Some(width),
        height: Some(height),
    };

    Ok(Json(response))
}

/// Parse a recipe URL to extract the recipe ID
fn parse_recipe_url(url: &str, base_url: &str) -> Option<Uuid> {
    // Expected format: {base_url}/recipes/{uuid}
    let path = url.strip_prefix(base_url)?;
    let path = path.strip_prefix("/recipes/")?;

    // Handle potential query strings or fragments
    let id_str = path.split(&['?', '#'][..]).next()?;

    Uuid::parse_str(id_str).ok()
}

/// Escape HTML special characters
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_recipe_url() {
        let base_url = "https://example.com";
        let id = Uuid::new_v4();
        let url = format!("{}/recipes/{}", base_url, id);

        assert_eq!(parse_recipe_url(&url, base_url), Some(id));
    }

    #[test]
    fn test_parse_recipe_url_with_query() {
        let base_url = "https://example.com";
        let id = Uuid::new_v4();
        let url = format!("{}/recipes/{}?foo=bar", base_url, id);

        assert_eq!(parse_recipe_url(&url, base_url), Some(id));
    }
}
