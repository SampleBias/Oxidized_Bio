use crate::llm::{LLMAdapter, LLMRequest, LLMResponse, AppResult};
use crate::types::{LLMMessage, TokenUsage};
use async_trait::async_trait;
use async_openai::Client;

pub struct OpenAIAdapter {
    client: Client,
}

impl OpenAIAdapter {
    pub fn new(api_key: &str) -> Self {
        let client = Client::new(api_key);
        Self { client }
    }
}

#[async_trait]
impl LLMAdapter for OpenAIAdapter {
    async fn create_chat_completion(&self, request: &LLMRequest) -> AppResult<LLMResponse> {
        let messages: Vec<async_openai::types::ChatCompletionRequestMessage> = request
            .messages
            .iter()
            .map(|m| match m.role.as_str() {
                "user" => async_openai::types::ChatCompletionRequestMessage::User {
                    content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(m.content.clone()),
                    name: None,
                },
                "assistant" => async_openai::types::ChatCompletionRequestMessage::Assistant {
                    content: Some(m.content.clone()),
                    name: None,
                    tool_calls: None,
                },
                "system" => async_openai::types::ChatCompletionRequestMessage::System {
                    content: m.content.clone(),
                    name: None,
                },
                _ => panic!("Unknown message role: {}", m.role),
            })
            .collect();

        let response = self
            .client
            .chat()
            .create(&async_openai::types::ChatCompletionRequestArgs::default()
                .model(&async_openai::types::ChatCompletionModel::from_string(&request.model)?)
                .messages(messages)
                .max_tokens(request.max_tokens)
                .temperature(request.temperature)
                .build()?
            )
            .await?;

        let content = response.choices[0].message.content.clone().unwrap_or_default();
        let usage = TokenUsage {
            prompt_tokens: response.usage.prompt_tokens as u32,
            completion_tokens: response.usage.completion_tokens as u32,
            total_tokens: response.usage.total_tokens as u32,
        };

        Ok(LLMResponse {
            content,
            finish_reason: response.choices[0].finish_reason.to_string(),
            usage,
        })
    }
}
