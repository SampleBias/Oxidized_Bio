use axum::{Router, routing::post, Json, extract::State};
use crate::models::AppState;
use tracing::info;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/files/{*path}", post(upload_file))
        .with_state(state)
}

async fn upload_file(
    State(_state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    info!("File upload request received");

    let response = serde_json::json!({
        "status": "success",
        "message": "File uploaded successfully"
    });

    Ok(Json(response))
}
