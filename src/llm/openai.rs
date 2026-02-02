use crate::llm::provider::LLMAdapter;
use crate::types::{AppResult, AppError, LLMRequest, LLMResponse, TokenUsage, MessageContent, ContentPart};
use async_trait::async_trait;
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::chat::{
        CreateChatCompletionRequestArgs,
        CreateChatCompletionRequest,
        ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent,
        ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent,
        ChatCompletionRequestUserMessageContentPart,
        ChatCompletionRequestAssistantMessage,
        ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessageContentPartText,
        ChatCompletionRequestMessageContentPartImage,
        ImageUrl,
        ImageDetail,
    },
};
use futures::StreamExt;
use futures::stream::BoxStream;

pub struct OpenAIAdapter {
    client: Client<OpenAIConfig>,
}

impl OpenAIAdapter {
    pub fn new(api_key: &str) -> Self {
        // Create config with the provided API key
        let config = OpenAIConfig::new().with_api_key(api_key);
        let client = Client::with_config(config);
        Self { client }
    }

    pub fn new_with_api_base(api_key: &str, api_base: &str) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(api_base);
        let client = Client::with_config(config);
        Self { client }
    }

    /// Convert internal ContentPart to OpenAI format
    fn convert_content_part(part: &ContentPart) -> ChatCompletionRequestUserMessageContentPart {
        match part {
            ContentPart::Text { text } => {
                ChatCompletionRequestUserMessageContentPart::Text(
                    ChatCompletionRequestMessageContentPartText {
                        text: text.clone(),
                    }
                )
            }
            ContentPart::ImageUrl { url, detail } => {
                ChatCompletionRequestUserMessageContentPart::ImageUrl(
                    ChatCompletionRequestMessageContentPartImage {
                        image_url: ImageUrl {
                            url: url.clone(),
                            detail: detail.as_ref().map(|d| match d.as_str() {
                                "low" => ImageDetail::Low,
                                "high" => ImageDetail::High,
                                _ => ImageDetail::Auto,
                            }),
                        },
                    }
                )
            }
            ContentPart::ImageBase64 { base64, media_type, detail } => {
                // Convert to data URL format for OpenAI
                let data_url = format!("data:{};base64,{}", media_type, base64);
                ChatCompletionRequestUserMessageContentPart::ImageUrl(
                    ChatCompletionRequestMessageContentPartImage {
                        image_url: ImageUrl {
                            url: data_url,
                            detail: detail.as_ref().map(|d| match d.as_str() {
                                "low" => ImageDetail::Low,
                                "high" => ImageDetail::High,
                                _ => ImageDetail::Auto,
                            }),
                        },
                    }
                )
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

    fn build_openai_request(request: &LLMRequest, stream: bool) -> AppResult<CreateChatCompletionRequest> {
        let mut messages: Vec<ChatCompletionRequestMessage> = request
            .messages
            .iter()
            .map(|m| match m.role.as_str() {
                "user" => ChatCompletionRequestMessage::User(
                    ChatCompletionRequestUserMessage {
                        content: Self::convert_user_content(&m.content),
                        name: None,
                    }
                ),
                "assistant" => ChatCompletionRequestMessage::Assistant(
                    ChatCompletionRequestAssistantMessage {
                        content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                            Self::get_text_content(&m.content)
                        )),
                        name: None,
                        tool_calls: None,
                        refusal: None,
                        function_call: None,
                        audio: None,
                    }
                ),
                "system" => ChatCompletionRequestMessage::System(
                    ChatCompletionRequestSystemMessage {
                        content: ChatCompletionRequestSystemMessageContent::Text(
                            Self::get_text_content(&m.content)
                        ),
                        name: None,
                    }
                ),
                _ => panic!("Unknown message role: {}", m.role),
            })
            .collect();

        if let Some(system) = request.system_instruction.as_ref() {
            messages.insert(0, ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessage {
                    content: ChatCompletionRequestSystemMessageContent::Text(system.clone()),
                    name: None,
                }
            ));
        }

        let mut request_builder = CreateChatCompletionRequestArgs::default();
        request_builder
            .model(&request.model)
            .messages(messages)
            .stream(stream);

        if let Some(max_tokens) = request.max_tokens {
            request_builder.max_tokens(max_tokens);
        }

        if let Some(temperature) = request.temperature {
            request_builder.temperature(temperature);
        }

        request_builder
            .build()
            .map_err(|e| AppError::LLMApi(format!("Failed to build request: {}", e)))
    }
}

#[async_trait]
impl LLMAdapter for OpenAIAdapter {
    #[allow(deprecated)]
    async fn create_chat_completion(&self, request: &LLMRequest) -> AppResult<LLMResponse> {
        let openai_request = Self::build_openai_request(request, false)?;

        let response = self
            .client
            .chat()
            .create(openai_request)
            .await
            .map_err(|e| AppError::LLMApi(format!("OpenAI API error: {}", e)))?;

        let content = response.choices.get(0)
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let usage = response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }).unwrap_or(TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });

        let finish_reason = response.choices.get(0)
            .and_then(|c| c.finish_reason.as_ref())
            .map(|r| format!("{:?}", r))
            .unwrap_or_else(|| "unknown".to_string());

        Ok(LLMResponse {
            content,
            finish_reason,
            usage,
        })
    }

    async fn create_chat_completion_stream(&self, request: &LLMRequest) -> AppResult<BoxStream<'static, AppResult<String>>> {
        let openai_request = Self::build_openai_request(request, true)?;

        let stream = self
            .client
            .chat()
            .create_stream(openai_request)
            .await
            .map_err(|e| AppError::LLMApi(format!("OpenAI API error: {}", e)))?;

        let mapped = stream.map(|chunk| {
            let chunk = chunk.map_err(|e| AppError::LLMApi(format!("OpenAI stream error: {}", e)))?;
            let delta = chunk.choices.get(0)
                .and_then(|c| c.delta.content.clone())
                .unwrap_or_default();
            Ok(delta)
        });

        Ok(Box::pin(mapped))
    }
}
