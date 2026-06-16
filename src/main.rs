use oppskrift::{app_router_with_rate_limit, core, AppState};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse CLI arguments
    let args: Vec<String> = std::env::args().collect();
    let should_seed = args.iter().any(|a| a == "--seed");

    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Health-check probe (used by the container HEALTHCHECK): query the running
    // server's /health endpoint and exit 0/1 without starting a second server.
    if args.iter().any(|a| a == "--health-check") {
        let port = std::env::var("LISTEN_PORT")
            .or_else(|_| std::env::var("PORT"))
            .unwrap_or_else(|_| "3000".to_string());
        let healthy = reqwest::get(format!("http://127.0.0.1:{port}/health"))
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false);
        if healthy {
            return Ok(());
        }
        std::process::exit(1);
    }

    // Validate configuration (panics if required vars missing)
    core::Config::from_env();

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
    let db = core::db::create_default_pool().await?;
    tracing::info!("Database connection pool created");

    // Run migrations (embedded in binary for production deployments)
    tracing::info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .expect("Failed to run database migrations");
    tracing::info!("Database migrations complete");

    // Run seeds if requested (then exit)
    if should_seed {
        match core::seeds::run(&db).await {
            Ok(result) => {
                tracing::info!(
                    "Seeding complete: {} users, {} recipes, {} books",
                    result.users,
                    result.recipes,
                    result.books
                );
                return Ok(());
            }
            Err(e) => {
                tracing::error!("Seeding failed: {}", e);
                return Err(e.into());
            }
        }
    }

    // Get CSRF secret from environment (required for security)
    let csrf_secret = std::env::var("CSRF_SECRET")
        .map(|s| s.into_bytes())
        .unwrap_or_else(|_| {
            tracing::warn!("CSRF_SECRET not set, generating random secret (not suitable for production clusters)");
            use rand::RngCore;
            let mut secret = vec![0u8; 32];
            rand::thread_rng().fill_bytes(&mut secret);
            secret
        });

    if csrf_secret.len() < 32 {
        tracing::error!("CSRF_SECRET must be at least 32 bytes");
        return Err(anyhow::anyhow!("CSRF_SECRET too short"));
    }

    // Create application state
    let state = AppState { db, csrf_secret };

    // Build the router with rate limiting enabled
    let app = app_router_with_rate_limit(state);

    // Get host and port from environment (LISTEN_* to avoid conflict with system HOST)
    let host = std::env::var("LISTEN_HOST")
        .or_else(|_| {
            std::env::var("HOST").map(|h| {
                if h.contains('.') || h == "localhost" {
                    h
                } else {
                    "0.0.0.0".to_string()
                }
            })
        })
        .unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("LISTEN_PORT")
        .or_else(|_| std::env::var("PORT"))
        .unwrap_or_else(|_| "3000".to_string());
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
