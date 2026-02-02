//! Oxidized Bio - AI Research Agent
//!
//! A high-performance AI agent framework for biological and scientific research.
//!
//! # Running Mode
//!
//! - **TUI Mode** (default): Interactive terminal interface
//!   ```bash
//!   oxidized-bio
//!   ```

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use oxidized_bio::{config::Config, tui};

/// Oxidized Bio - AI Research Agent for biological and scientific research
#[derive(Parser, Debug)]
#[command(name = "oxidized-bio")]
#[command(author, version, about, long_about = None)]
struct Cli {
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
    // IMPORTANT: In TUI mode, we must NOT write logs to stdout/stderr as it corrupts
    // the alternate screen display. Instead, we either write to a log file or disable
    // logging entirely.
    if cli.verbose {
        // Server/verbose mode - write logs to stdout
        let log_level = if cli.verbose {
            "oxidized_bio=debug,tower_http=debug,axum=debug"
        } else {
            "oxidized_bio=info,tower_http=info"
        };

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| log_level.into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    } else {
        // TUI mode - write logs to a file to avoid corrupting the display
        // Create log directory if needed
        let log_dir = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("oxidized-bio")
            .join("logs");
        
        if let Err(_) = std::fs::create_dir_all(&log_dir) {
            // If we can't create log dir, just disable logging entirely for TUI
            // This is better than corrupting the display
            tracing_subscriber::registry()
                .with(tracing_subscriber::EnvFilter::new("off"))
                .init();
        } else {
            // Create log file with timestamp
            let log_file = log_dir.join(format!(
                "oxidized-bio-{}.log",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            ));
            
            match std::fs::File::create(&log_file) {
                Ok(file) => {
                    // Write logs to file instead of stdout
                    let file_layer = tracing_subscriber::fmt::layer()
                        .with_writer(std::sync::Mutex::new(file))
                        .with_ansi(false); // No ANSI colors in log file
                    
                    tracing_subscriber::registry()
                        .with(
                            tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "oxidized_bio=info".into()),
                )
                .with(file_layer)
                .init();
            }
            Err(_) => {
                // Fall back to no logging if file creation fails
                tracing_subscriber::registry()
                    .with(tracing_subscriber::EnvFilter::new("off"))
                    .init();
            }
        }
    }
    }

    // Load configuration
    let config = Config::from_env()?;

    run_tui(config).await
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

// Server mode removed. This build runs as a single-user TUI application.
