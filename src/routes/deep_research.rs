use axum::{
    Router,
    routing::{get, post},
    Json,
    extract::{State, Path},
    response::Json as ResponseJson,
};
use crate::models::{AppState, DeepResearchRequest, DeepResearchResponse};
use uuid::Uuid;
use tracing::info;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/deep-research/start", post(start_deep_research))
        .route("/api/deep-research/status/{message_id}", get(get_status))
        .with_state(state)
}

async fn start_deep_research(
    State(state): State<AppState>,
    Json(request): Json<DeepResearchRequest>,
) -> Result<ResponseJson<DeepResearchResponse>, axum::http::StatusCode> {
    info!("Received deep research request: {:?}", request.message);

    let message_id = Uuid::new_v4();
    let conversation_id = request.conversation_id.unwrap_or_else(|| Uuid::new_v4());

    let response = DeepResearchResponse {
        message_id,
        status: "pending".to_string(), // Would be "processing" or "completed"
        conversation_id,
    };

    Ok(Json(response))
}

async fn get_status(
    State(_state): State<AppState>,
    Path(message_id): Path<Uuid>,
) -> Result<ResponseJson<serde_json::Value>, axum::http::StatusCode> {
    info!("Checking status for message: {:?}", message_id);

    let status = serde_json::json!({
        "message_id": message_id,
        "status": "completed",
        "progress": 100
    });

    Ok(Json(status))
}
