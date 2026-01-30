use axum::{
    Router,
    routing::{get, post},
    Json,
    extract::State,
    response::Json as ResponseJson,
};
use crate::models::{AppState, ChatRequest, ChatResponse};
use uuid::Uuid;
use tracing::info;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/chat", get(get_chat))
        .route("/api/chat", post(post_chat))
        .with_state(state)
}

async fn get_chat(
    State(state): State<AppState>,
) -> Result<ResponseJson<&'static str>, axum::http::StatusCode> {
    Ok(Json("Chat endpoint - GET method returns this message"))
}

pub async fn post_chat(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Result<ResponseJson<ChatResponse>, axum::http::StatusCode> {
    info!("Received chat request: {:?}", request.message);

    // Get or create user (placeholder - would use JWT or wallet from headers)
    let user_id = Uuid::new_v4();

    // Get or create conversation
    let conversation_id = request.conversation_id.unwrap_or_else(|| {
        // This would create a new conversation in production
        Uuid::new_v4()
    });

    // Placeholder: Process the chat message
    // In production, this would:
    // 1. Create a message record
    // 2. Call the planning agent
    // 3. Execute planned tasks (literature search, analysis, etc.)
    // 4. Generate a reply
    // 5. Update conversation state

    let response = ChatResponse {
        message_id: Uuid::new_v4(),
        content: format!("Echo: {}", request.message), // Placeholder response
        conversation_id,
        response_time: 100, // Placeholder
    };

    info!("Chat response sent: {:?}", response.message_id);

    Ok(Json(response))
}
