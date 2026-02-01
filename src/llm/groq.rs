use crate::llm::provider::LLMAdapter;
use crate::types::{AppResult, LLMRequest, LLMResponse};
use async_trait::async_trait;

const GROQ_API_BASE: &str = "https://api.groq.com/openai/v1";

pub struct GroqAdapter {
    inner: crate::llm::openai::OpenAIAdapter,
}

impl GroqAdapter {
    pub fn new(api_key: &str) -> Self {
        Self {
            inner: crate::llm::openai::OpenAIAdapter::new_with_api_base(api_key, GROQ_API_BASE),
        }
    }
}

#[async_trait]
impl LLMAdapter for GroqAdapter {
    async fn create_chat_completion(&self, request: &LLMRequest) -> AppResult<LLMResponse> {
        self.inner.create_chat_completion(request).await
    }
}
