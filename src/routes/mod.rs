//! API Routes
//! 
//! This module organizes all HTTP endpoints for the application:
//! - `/api/chat` - Main chat endpoint
//! - `/api/deep-research` - Deep research mode
//! - `/api/files` - File upload handling
//! - `/api/health` - Health checks
//! - `/api/settings` - User settings and API key management
//! - `/api/rfc` - Remote Function Call endpoints
//! - `/` - Static file serving (frontend)

pub mod chat;
pub mod deep_research;
pub mod health;
pub mod files;
pub mod static_files;

use axum::Router;
use crate::models::AppState;
use crate::rfc;
use crate::settings;
use tracing::info;

/// Create the main application router
/// 
/// Routes are organized as follows:
/// - API routes are prefixed with `/api/`
/// - Static files are served from root `/`
/// - Fallback serves index.html for SPA routing
pub fn create_router(state: AppState) -> Router {
    info!("Creating application router");
    
    // API routes (with state)
    let api_router = Router::new()
        .merge(chat::router(state.clone()))
        .merge(deep_research::router(state.clone()))
        .merge(files::router(state.clone()))
        .merge(rfc::router(state))
        .merge(health::router())
        .merge(settings::router());  // Settings API (no state needed)

    // Combine API routes with static file serving
    // API routes take precedence over static files
    Router::new()
        .merge(api_router)
        .merge(static_files::router())
}
