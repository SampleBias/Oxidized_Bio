//! Chat Route
//! 
//! Handles the main chat endpoint that powers the research assistant.
//! This endpoint orchestrates the full agent pipeline:
//! Planning → Literature Search → Reply Generation

use axum::{
    Router,
    routing::{get, post},
    Json,
    extract::State,
    response::Json as ResponseJson,
    http::StatusCode,
};
use crate::models::{AppState, ChatRequest, ChatResponse};
use crate::agents;
use uuid::Uuid;
use tracing::{info, error};
use std::time::Instant;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/chat", get(get_chat))
        .route("/api/chat", post(post_chat))
        .with_state(state)
}

/// GET /api/chat - Returns endpoint info
async fn get_chat() -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "endpoint": "/api/chat",
        "method": "POST",
        "description": "Send a message to the research assistant",
        "request_format": {
            "message": "string (required)",
            "conversation_id": "uuid (optional)",
            "files": "array (optional)"
        }
    })))
}

/// POST /api/chat - Process a chat message through the agent pipeline
pub async fn post_chat(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Result<ResponseJson<ChatResponse>, StatusCode> {
    let start_time = Instant::now();
    let message_id = Uuid::new_v4();
    
    info!(
        message_id = %message_id,
        message_len = request.message.len(),
        has_files = request.files.is_some(),
        "Received chat request"
    );

    // Get or create conversation
    let conversation_id = request.conversation_id.unwrap_or_else(|| {
        let new_id = Uuid::new_v4();
        info!(conversation_id = %new_id, "Created new conversation");
        new_id
    });

    // Execute the research pipeline
    let response_text = match agents::execute_research_pipeline(
        &request.message,
        None, // TODO: Load conversation state from DB
        &state.config,
    ).await {
        Ok(text) => text,
        Err(e) => {
            error!(error = %e, "Research pipeline failed");
            format!(
                "I apologize, but I encountered an error processing your request: {}\n\n\
                Please try again or rephrase your question.",
                e
            )
        }
    };

    let elapsed = start_time.elapsed();
    info!(
        message_id = %message_id,
        elapsed_ms = elapsed.as_millis() as u64,
        response_len = response_text.len(),
        "Chat response generated"
    );

    // Build response in the format the frontend expects
    let response = ChatResponse {
        text: response_text,
        user_id: None, // Set for x402 users
        message_id: Some(message_id),
        files: None,
    };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_response_serialization() {
        let response = ChatResponse {
            text: "Hello, world!".to_string(),
            user_id: None,
            message_id: Some(Uuid::new_v4()),
            files: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"text\""));
        assert!(json.contains("Hello, world!"));
    }
}
