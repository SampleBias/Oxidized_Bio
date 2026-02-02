// Google adapter stub
// TODO: Implement full Google adapter

use crate::llm::provider::LLMAdapter;
use crate::types::{AppResult, LLMRequest, LLMResponse, AppError};
use async_trait::async_trait;
use futures::stream::BoxStream;

pub struct GoogleAdapter {
    api_key: String,
}

impl GoogleAdapter {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
        }
    }
}

#[async_trait]
impl LLMAdapter for GoogleAdapter {
    async fn create_chat_completion(&self, _request: &LLMRequest) -> AppResult<LLMResponse> {
        // Placeholder implementation
        Ok(LLMResponse {
            content: String::new(),
            finish_reason: "STOP".to_string(),
            usage: crate::types::TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
        })
    }

    async fn create_chat_completion_stream(&self, _request: &LLMRequest) -> AppResult<BoxStream<'static, AppResult<String>>> {
        Err(AppError::LLMApi("Streaming not supported for Google adapter".to_string()))
    }
}
