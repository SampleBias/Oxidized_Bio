//! Oxidized Bio - AI Research Agent
//!
//! A high-performance AI agent framework for biological and scientific research.
//!
//! # Running Modes
//!
//! - **TUI Mode** (default): Interactive terminal interface
//!   ```bash
//!   oxidized-bio
//!   ```
//!
//! - **Server Mode**: HTTP API server
//!   ```bash
//!   oxidized-bio --server
//!   oxidized-bio --server --port 8080
//!   ```

use clap::Parser;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use oxidized_bio::{config::Config, routes::create_router, tui};

/// Oxidized Bio - AI Research Agent for biological and scientific research
#[derive(Parser, Debug)]
#[command(name = "oxidized-bio")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Run as HTTP API server instead of TUI
    #[arg(short, long)]
    server: bool,

    /// Server port (only with --server)
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing based on mode
    let log_level = if cli.verbose {
        "oxidized_bio=debug,tower_http=debug,axum=debug"
    } else if cli.server {
        "oxidized_bio=info,tower_http=info"
    } else {
        // TUI mode - minimal logging to not interfere with display
        "oxidized_bio=warn"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;

    if cli.server {
        run_server(config, cli.port).await
    } else {
        run_tui(config).await
    }
}

/// Run in TUI mode (default)
async fn run_tui(config: Config) -> anyhow::Result<()> {
    // For TUI mode, we don't need the database connection at startup
    // The agents will use the config to make API calls directly

    // Run the TUI
    let result = tui::run(config).await;

    // Handle any errors from TUI
    if let Err(ref e) = result {
        // Print error to stderr after terminal is restored
        eprintln!("Error: {}", e);
    }

    result
}

/// Run in HTTP server mode
async fn run_server(config: Config, port: u16) -> anyhow::Result<()> {
    info!("Starting Oxidized Bio server...");
    info!("Configuration loaded: {:?}", config.server);

    // Check if DATABASE_URL is configured
    if config.database.url.is_empty() {
        error!("DATABASE_URL is required for server mode");
        error!("Set DATABASE_URL environment variable, e.g.:");
        error!("  DATABASE_URL=postgresql://user:pass@localhost:5432/oxidized_bio");
        error!("");
        error!("Or use TUI mode (no database required): oxidized-bio");
        return Err(anyhow::anyhow!(
            "DATABASE_URL is required for server mode. Use TUI mode for database-free operation."
        ));
    }

    // Connect to database
    let pool = oxidized_bio::db::create_pool(&config.database).await?;

    // Run migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
    info!("Database migrations completed");

    // Create shared state
    let state = oxidized_bio::AppState {
        pool,
        config: config.clone(),
    };

    // Create router
    let app = create_router(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Server listening on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}
