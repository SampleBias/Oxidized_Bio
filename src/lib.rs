// Oxidized Bio - High-performance AI agent framework for biological research

pub mod config;
pub mod db;
pub mod models;
pub mod types;
pub mod agents;
pub mod llm;
pub mod search;    // Search APIs (SerpAPI for Google Scholar and Light)
pub mod embeddings;
pub mod storage;
pub mod routes;
pub mod middleware;
pub mod queue;
pub mod payment;
pub mod utils;
pub mod rfc;       // Remote Function Call system for Docker container access
pub mod settings;  // User settings and API key management
pub mod tui;       // Terminal User Interface
pub mod data_registry;
pub mod analysis;

// Re-exports for convenience
pub use config::Config;
pub use models::AppState;
// Note: Import specific items from types module instead of glob to avoid name conflicts
// e.g., use oxidized_bio::types::{LLMRequest, LLMResponse, AppResult};

pub fn create_router(state: AppState) -> axum::Router {
    routes::create_router(state)
}
