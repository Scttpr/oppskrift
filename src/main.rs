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
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,oppskrift=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create database connection pool
    let db = lib::db::create_default_pool().await?;
    tracing::info!("Database connection pool created");

    // Create application state
    let state = AppState { db };

    // Build the router
    let app = create_router(state);

    // Get host and port from environment
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a valid number");

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    tracing::info!("Listening on http://{}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

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
        // HTML handler routes will be mounted here
        // .nest("/", handlers::routes())
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
