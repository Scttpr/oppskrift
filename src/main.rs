#![allow(special_module_name)]

use axum::{routing::get, Router};
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod handlers;
mod jobs;
mod lib;
mod models;
mod services;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let args: Vec<String> = std::env::args().collect();
    let should_seed = args.iter().any(|a| a == "--seed");

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Validate configuration (panics if required vars missing)
    lib::Config::from_env();

    // Initialize tracing with JSON format in production
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,oppskrift=debug,tower_http=debug".into());

    let is_production = std::env::var("RUST_ENV")
        .map(|v| v == "production")
        .unwrap_or(false);

    if is_production {
        // JSON structured logging for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        // Human-readable logging for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    // Create database connection pool
    let db = lib::db::create_default_pool().await?;
    tracing::info!("Database connection pool created");

    // Run seeds if requested
    if should_seed {
        match lib::seeds::run(&db).await {
            Ok(result) => {
                tracing::info!(
                    "Seeding complete: {} users, {} recipes, {} books",
                    result.users,
                    result.recipes,
                    result.books
                );
            }
            Err(e) => {
                tracing::error!("Seeding failed: {}", e);
                return Err(e.into());
            }
        }
    }

    // Create application state
    let state = AppState { db };

    // Build the router
    let app = create_router(state);

    // Get host and port from environment
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    tracing::info!("Listening on http://{}", addr);

    // Start the server with connect info for client IP extraction
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

/// Create the application router with all middleware
fn create_router(state: AppState) -> Router {
    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // API routes
        .nest("/api/v1", api::routes())
        // ActivityPub federation routes
        .merge(api::federation_routes())
        // Content syndication routes (RSS, Atom, oEmbed)
        .merge(api::syndication_routes())
        // API documentation
        .merge(api::docs_routes())
        // HTML handler routes
        .merge(handlers::routes())
        // Static file serving
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}
