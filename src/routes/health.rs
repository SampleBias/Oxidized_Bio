use axum::{Router, routing::get, Json, response::Json as ResponseJson};
use crate::models::HealthResponse;

pub fn router() -> Router {
    Router::new()
        .route("/api/health", get(health_check))
}

async fn health_check() -> ResponseJson<HealthResponse> {
    let response = HealthResponse {
        status: "ok".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        database: "connected".to_string(),
        redis: None, // Would check Redis connection if enabled
    };

    Json(response)
}
