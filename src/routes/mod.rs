// API routes

pub mod chat;
pub mod deep_research;
pub mod health;
pub mod files;

use axum::Router;
use crate::models::AppState;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .merge(chat::router(state.clone()))
        .merge(deep_research::router(state.clone()))
        .merge(health::router())
        .merge(files::router(state.clone()))
        .with_state(state)
}
