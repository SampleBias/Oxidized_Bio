// Oxidized Bio - High-performance AI agent framework for biological research

pub mod config;
pub mod db;
pub mod models;
pub mod types;
pub mod agents;
pub mod llm;
pub mod embeddings;
pub mod storage;
pub mod routes;
pub mod middleware;
pub mod queue;
pub mod payment;
pub mod utils;

use std::sync::Arc;
use sqlx::PgPool;
use config::Config;
use models::AppState;

// Re-exports for convenience
pub use config::Config;
pub use models::AppState;
pub use types::*;

pub fn create_router(state: AppState) -> axum::Router {
    routes::create_router(state)
}
