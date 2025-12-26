//! RSS/Atom feed endpoints
//!
//! Provides syndication feeds for recipes and user profiles.

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chrono::Utc;
use uuid::Uuid;

use crate::lib::pagination::PaginationParams;
use crate::services::{RecipeService, UserService};
use crate::AppState;

/// Feed routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/feeds/recipes.rss", get(recipes_rss))
        .route("/feeds/recipes.atom", get(recipes_atom))
        .route("/feeds/users/:id/recipes.rss", get(user_recipes_rss))
        .route("/feeds/users/:id/recipes.atom", get(user_recipes_atom))
}

/// RSS feed for public recipes
async fn recipes_rss(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Response, StatusCode> {
    let recipes = RecipeService::list_public(&state.db, &params)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let items: Vec<String> = recipes
        .data
        .iter()
        .map(|r| {
            format!(
                r#"    <item>
      <title>{}</title>
      <link>{}/recipes/{}</link>
      <description>{}</description>
      <pubDate>{}</pubDate>
      <guid>{}/recipes/{}</guid>
    </item>"#,
                escape_xml(&r.title),
                base_url,
                r.id,
                escape_xml(&r.description.clone().unwrap_or_default()),
                r.created_at.format("%a, %d %b %Y %H:%M:%S GMT"),
                base_url,
                r.id
            )
        })
        .collect();

    let rss = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Oppskrift - Public Recipes</title>
    <link>{}</link>
    <description>Latest public recipes from Oppskrift</description>
    <language>en</language>
    <lastBuildDate>{}</lastBuildDate>
{}
  </channel>
</rss>"#,
        base_url,
        Utc::now().format("%a, %d %b %Y %H:%M:%S GMT"),
        items.join("\n")
    );

    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        rss,
    )
        .into_response())
}

/// Atom feed for public recipes
async fn recipes_atom(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Response, StatusCode> {
    let recipes = RecipeService::list_public(&state.db, &params)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let entries: Vec<String> = recipes
        .data
        .iter()
        .map(|r| {
            format!(
                r#"  <entry>
    <title>{}</title>
    <link href="{}/recipes/{}" />
    <id>{}/recipes/{}</id>
    <updated>{}</updated>
    <summary>{}</summary>
  </entry>"#,
                escape_xml(&r.title),
                base_url,
                r.id,
                base_url,
                r.id,
                r.created_at.to_rfc3339(),
                escape_xml(&r.description.clone().unwrap_or_default())
            )
        })
        .collect();

    let atom = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Oppskrift - Public Recipes</title>
  <link href="{}" />
  <link href="{}/feeds/recipes.atom" rel="self" />
  <id>{}/feeds/recipes.atom</id>
  <updated>{}</updated>
{}
</feed>"#,
        base_url,
        base_url,
        base_url,
        Utc::now().to_rfc3339(),
        entries.join("\n")
    );

    Ok((
        [(header::CONTENT_TYPE, "application/atom+xml; charset=utf-8")],
        atom,
    )
        .into_response())
}

/// RSS feed for a user's public recipes
async fn user_recipes_rss(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<Response, StatusCode> {
    let user = UserService::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let recipes = RecipeService::list_by_author(&state.db, id, &params)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let display_name = &user.display_name;

    let items: Vec<String> = recipes
        .data
        .iter()
        .map(|r| {
            format!(
                r#"    <item>
      <title>{}</title>
      <link>{}/recipes/{}</link>
      <description>{}</description>
      <pubDate>{}</pubDate>
      <guid>{}/recipes/{}</guid>
    </item>"#,
                escape_xml(&r.title),
                base_url,
                r.id,
                escape_xml(&r.description.clone().unwrap_or_default()),
                r.created_at.format("%a, %d %b %Y %H:%M:%S GMT"),
                base_url,
                r.id
            )
        })
        .collect();

    let rss = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>{}'s Recipes - Oppskrift</title>
    <link>{}/users/{}</link>
    <description>Recipes by {}</description>
    <language>en</language>
    <lastBuildDate>{}</lastBuildDate>
{}
  </channel>
</rss>"#,
        escape_xml(display_name),
        base_url,
        id,
        escape_xml(display_name),
        Utc::now().format("%a, %d %b %Y %H:%M:%S GMT"),
        items.join("\n")
    );

    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        rss,
    )
        .into_response())
}

/// Atom feed for a user's public recipes
async fn user_recipes_atom(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<Response, StatusCode> {
    let user = UserService::get_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let recipes = RecipeService::list_by_author(&state.db, id, &params)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let display_name = &user.display_name;

    let entries: Vec<String> = recipes
        .data
        .iter()
        .map(|r| {
            format!(
                r#"  <entry>
    <title>{}</title>
    <link href="{}/recipes/{}" />
    <id>{}/recipes/{}</id>
    <updated>{}</updated>
    <summary>{}</summary>
  </entry>"#,
                escape_xml(&r.title),
                base_url,
                r.id,
                base_url,
                r.id,
                r.created_at.to_rfc3339(),
                escape_xml(&r.description.clone().unwrap_or_default())
            )
        })
        .collect();

    let atom = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>{}'s Recipes - Oppskrift</title>
  <link href="{}/users/{}" />
  <link href="{}/feeds/users/{}/recipes.atom" rel="self" />
  <id>{}/feeds/users/{}/recipes.atom</id>
  <updated>{}</updated>
  <author>
    <name>{}</name>
  </author>
{}
</feed>"#,
        escape_xml(display_name),
        base_url,
        id,
        base_url,
        id,
        base_url,
        id,
        Utc::now().to_rfc3339(),
        escape_xml(display_name),
        entries.join("\n")
    );

    Ok((
        [(header::CONTENT_TYPE, "application/atom+xml; charset=utf-8")],
        atom,
    )
        .into_response())
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
