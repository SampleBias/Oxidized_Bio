use async_trait::async_trait;
use crate::types::{LLMRequest, LLMResponse, AppResult};

#[async_trait]
pub trait LLMAdapter: Send + Sync {
    async fn create_chat_completion(&self, request: &LLMRequest) -> AppResult<LLMResponse>;
}

/// Configuration for LLM provider (renamed to avoid conflict with LLMProvider enum in types.rs)
pub struct LLMProviderConfig {
    pub name: String,
    pub api_key: String,
}

pub struct LLM {
    adapter: Box<dyn LLMAdapter>,
    provider_name: String,
}

impl LLM {
    pub fn new(provider: LLMProviderConfig) -> Self {
        let adapter: Box<dyn LLMAdapter> = match provider.name.as_str() {
            "openai" => Box::new(crate::llm::openai::OpenAIAdapter::new(&provider.api_key)),
            "anthropic" => Box::new(crate::llm::anthropic::AnthropicAdapter::new(&provider.api_key)),
            "google" => Box::new(crate::llm::google::GoogleAdapter::new(&provider.api_key)),
            "openrouter" => Box::new(crate::llm::openrouter::OpenRouterAdapter::new(&provider.api_key)),
            "glm" => Box::new(crate::llm::glm::GLMAdapter::new(&provider.api_key)),
            "glm-general" => Box::new(crate::llm::glm::GLMAdapter::with_general_api(&provider.api_key)),
            _ => panic!("Unsupported provider: {}", provider.name),
        };

        Self {
            adapter,
            provider_name: provider.name,
        }
    }

    pub async fn create_chat_completion(&self, request: &LLMRequest) -> AppResult<LLMResponse> {
        self.adapter.create_chat_completion(request).await
    }
}
