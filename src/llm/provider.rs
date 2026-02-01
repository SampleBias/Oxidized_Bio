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
            // GLM General API - for general use, vision models, and users without Coding Plan
            // Endpoint: https://api.z.ai/api/paas/v4
            "glm" | "glm-general" => Box::new(crate::llm::glm::GLMAdapter::new(&provider.api_key)),
            // GLM Coding API - requires GLM Coding Plan subscription
            // Endpoint: https://api.z.ai/api/coding/paas/v4
            // Use for coding scenarios with GLM-4.7 in tools like Claude Code, Cline, etc.
            "glm-coding" => Box::new(crate::llm::glm::GLMAdapter::with_coding_api(&provider.api_key)),
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
