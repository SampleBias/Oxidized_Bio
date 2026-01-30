use crate::llm::provider::LLMAdapter;
use crate::types::{AppResult, LLMRequest, LLMResponse, TokenUsage, MessageContent, ContentPart};
use async_trait::async_trait;
use async_openai::Client;
use async_openai::types::{
    ChatCompletionRequestMessage, ChatCompletionRequestUserMessageContent,
    ChatCompletionRequestUserMessageContentPart, ImageUrl, ImageDetail,
};

pub struct OpenAIAdapter {
    client: Client,
}

impl OpenAIAdapter {
    pub fn new(api_key: &str) -> Self {
        let client = Client::new(api_key);
        Self { client }
    }

    /// Convert internal ContentPart to OpenAI format
    fn convert_content_part(part: &ContentPart) -> ChatCompletionRequestUserMessageContentPart {
        match part {
            ContentPart::Text { text } => {
                ChatCompletionRequestUserMessageContentPart::Text(text.clone())
            }
            ContentPart::ImageUrl { url, detail } => {
                ChatCompletionRequestUserMessageContentPart::ImageUrl(ImageUrl {
                    url: url.clone(),
                    detail: detail.as_ref().map(|d| match d.as_str() {
                        "low" => ImageDetail::Low,
                        "high" => ImageDetail::High,
                        _ => ImageDetail::Auto,
                    }),
                })
            }
            ContentPart::ImageBase64 { base64, media_type, detail } => {
                // Convert to data URL format for OpenAI
                let data_url = format!("data:{};base64,{}", media_type, base64);
                ChatCompletionRequestUserMessageContentPart::ImageUrl(ImageUrl {
                    url: data_url,
                    detail: detail.as_ref().map(|d| match d.as_str() {
                        "low" => ImageDetail::Low,
                        "high" => ImageDetail::High,
                        _ => ImageDetail::Auto,
                    }),
                })
            }
        }
    }

    /// Convert MessageContent to OpenAI user message content
    fn convert_user_content(content: &MessageContent) -> ChatCompletionRequestUserMessageContent {
        match content {
            MessageContent::Text(text) => {
                ChatCompletionRequestUserMessageContent::Text(text.clone())
            }
            MessageContent::Multimodal(parts) => {
                let openai_parts: Vec<ChatCompletionRequestUserMessageContentPart> = parts
                    .iter()
                    .map(Self::convert_content_part)
                    .collect();
                ChatCompletionRequestUserMessageContent::Array(openai_parts)
            }
        }
    }

    /// Get text content from MessageContent (for assistant/system messages)
    fn get_text_content(content: &MessageContent) -> String {
        match content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Multimodal(parts) => {
                // Concatenate all text parts
                parts
                    .iter()
                    .filter_map(|p| match p {
                        ContentPart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    }
}

#[async_trait]
impl LLMAdapter for OpenAIAdapter {
    async fn create_chat_completion(&self, request: &LLMRequest) -> AppResult<LLMResponse> {
        let messages: Vec<ChatCompletionRequestMessage> = request
            .messages
            .iter()
            .map(|m| match m.role.as_str() {
                "user" => ChatCompletionRequestMessage::User {
                    content: Self::convert_user_content(&m.content),
                    name: None,
                },
                "assistant" => ChatCompletionRequestMessage::Assistant {
                    content: Some(Self::get_text_content(&m.content)),
                    name: None,
                    tool_calls: None,
                },
                "system" => ChatCompletionRequestMessage::System {
                    content: Self::get_text_content(&m.content),
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
