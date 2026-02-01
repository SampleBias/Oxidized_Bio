// GLM (Zhipu AI) adapter implementation
// Supports GLM-4.7 series (text) and GLM-4.6V series (vision/multimodal)
// Documentation: https://docs.z.ai/guides/overview/quick-start
// API Reference: https://docs.z.ai/api-reference/llm/chat-completion

use crate::llm::provider::LLMAdapter;
use crate::types::{AppError, AppResult, LLMRequest, LLMResponse, TokenUsage, MessageContent};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

// All GLM models use the same API endpoint (verified from official docs)
const GLM_API_BASE: &str = "https://api.z.ai/api/paas/v4";

pub struct GLMAdapter {
    client: Client,
    api_key: String,
}

// Request types for GLM API
#[derive(Serialize)]
struct GLMChatRequest {
    model: String,
    messages: Vec<GLMMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Serialize)]
struct GLMMessage {
    role: String,
    content: GLMMessageContent,
}

// GLM supports both simple text and multimodal content (like OpenAI format)
#[derive(Serialize)]
#[serde(untagged)]
enum GLMMessageContent {
    Text(String),
    Multimodal(Vec<GLMContentPart>),
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum GLMContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: GLMImageUrl },
}

#[derive(Serialize)]
struct GLMImageUrl {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>, // "low", "high", or "auto"
}

// Response types for GLM API
#[derive(Deserialize)]
struct GLMChatResponse {
    id: String,
    model: String,
    choices: Vec<GLMChoice>,
    usage: GLMUsage,
}

#[derive(Deserialize)]
struct GLMChoice {
    index: u32,
    message: GLMResponseMessage,
    finish_reason: String,
}

#[derive(Deserialize)]
struct GLMResponseMessage {
    role: String,
    content: String,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct GLMUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    #[serde(default)]
    cached_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct GLMErrorResponse {
    error: GLMError,
}

#[derive(Deserialize)]
struct GLMError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
    code: Option<String>,
}

impl GLMAdapter {
    /// Create a new GLM adapter
    /// All GLM models (text and vision) use the same API endpoint
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
        }
    }

    /// Alias for new() - kept for backwards compatibility
    #[deprecated(note = "All GLM models use the same API endpoint. Use new() instead.")]
    pub fn with_general_api(api_key: &str) -> Self {
        Self::new(api_key)
    }

    /// Alias for new() - kept for backwards compatibility
    #[deprecated(note = "All GLM models use the same API endpoint. Use new() instead.")]
    pub fn with_api_type(api_key: &str, _use_coding_api: bool) -> Self {
        Self::new(api_key)
    }

    /// Check if a model is a vision model (requires multimodal content format)
    /// Vision models: glm-4.6v, glm-4.6v-flashx, glm-4.6v-flash, glm-4.5v, autoglm-*
    fn is_vision_model(model: &str) -> bool {
        let model_lower = model.to_lowercase();
        // GLM vision models have "v" suffix (e.g., glm-4.6v, glm-4.5v)
        // or are AutoGLM models (phone/multilingual)
        model_lower.contains(".6v") || 
        model_lower.contains(".5v") || 
        model_lower.contains("-v-") ||
        model_lower.ends_with("v") ||
        model_lower.contains("vision") ||
        model_lower.contains("autoglm")
    }

    /// Convert internal message format to GLM API format
    fn convert_message(msg: &crate::types::LLMMessage) -> GLMMessage {
        let content = match &msg.content {
            MessageContent::Text(text) => GLMMessageContent::Text(text.clone()),
            MessageContent::Multimodal(parts) => {
                let glm_parts: Vec<GLMContentPart> = parts
                    .iter()
                    .map(|part| match part {
                        crate::types::ContentPart::Text { text } => {
                            GLMContentPart::Text { text: text.clone() }
                        }
                        crate::types::ContentPart::ImageUrl { url, detail } => {
                            GLMContentPart::ImageUrl {
                                image_url: GLMImageUrl {
                                    url: url.clone(),
                                    detail: detail.clone(),
                                },
                            }
                        }
                        crate::types::ContentPart::ImageBase64 { base64, media_type, detail } => {
                            // Convert base64 to data URL format
                            let data_url = format!("data:{};base64,{}", media_type, base64);
                            GLMContentPart::ImageUrl {
                                image_url: GLMImageUrl {
                                    url: data_url,
                                    detail: detail.clone(),
                                },
                            }
                        }
                    })
                    .collect();
                GLMMessageContent::Multimodal(glm_parts)
            }
        };

        GLMMessage {
            role: msg.role.clone(),
            content,
        }
    }
}

#[async_trait]
impl LLMAdapter for GLMAdapter {
    async fn create_chat_completion(&self, request: &LLMRequest) -> AppResult<LLMResponse> {
        let url = format!("{}/chat/completions", GLM_API_BASE);

        let messages: Vec<GLMMessage> = request
            .messages
            .iter()
            .map(Self::convert_message)
            .collect();

        let glm_request = GLMChatRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: Some(false),
            top_p: None,
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&glm_request)
            .send()
            .await
            .map_err(|e| AppError::LLMApi(format!("GLM request failed: {}", e)))?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            
            // Try to parse as GLM error response
            if let Ok(error_response) = serde_json::from_str::<GLMErrorResponse>(&error_text) {
                return Err(AppError::LLMApi(format!(
                    "GLM API error ({}): {} (code: {:?})",
                    status,
                    error_response.error.message,
                    error_response.error.code
                )));
            }
            
            return Err(AppError::LLMApi(format!(
                "GLM API error ({}): {}",
                status, error_text
            )));
        }

        let glm_response: GLMChatResponse = response
            .json()
            .await
            .map_err(|e| AppError::LLMApi(format!("Failed to parse GLM response: {}", e)))?;

        let choice = glm_response
            .choices
            .first()
            .ok_or_else(|| AppError::LLMApi("GLM returned no choices".to_string()))?;

        // Combine regular content with reasoning content if present
        let content = if let Some(reasoning) = &choice.message.reasoning_content {
            if !reasoning.is_empty() {
                format!("{}\n\n---\nReasoning:\n{}", choice.message.content, reasoning)
            } else {
                choice.message.content.clone()
            }
        } else {
            choice.message.content.clone()
        };

        Ok(LLMResponse {
            content,
            finish_reason: choice.finish_reason.clone(),
            usage: TokenUsage {
                prompt_tokens: glm_response.usage.prompt_tokens,
                completion_tokens: glm_response.usage.completion_tokens,
                total_tokens: glm_response.usage.total_tokens,
            },
        })
    }
}

/// Available GLM models (verified from official docs: https://docs.z.ai)
/// All models use the same API endpoint: https://api.z.ai/api/paas/v4
pub mod models {
    // === GLM-4.7 Series (Latest flagship text models) ===
    /// GLM-4.7 - Flagship model with highest performance (200K context, 128K output)
    pub const GLM_4_7: &str = "glm-4.7";
    /// GLM-4.7-FlashX - Lightweight, high-speed, affordable (200K context, 128K output)
    pub const GLM_4_7_FLASHX: &str = "glm-4.7-flashx";
    /// GLM-4.7-Flash - Lightweight, completely free (200K context, 128K output)
    pub const GLM_4_7_FLASH: &str = "glm-4.7-flash";
    
    // === GLM-4.6 Series (Previous generation text models) ===
    pub const GLM_4_6: &str = "glm-4.6";
    
    // === GLM-4.5 Series (Open-source models) ===
    pub const GLM_4_5: &str = "glm-4.5";
    
    // === Extended Context Models ===
    pub const GLM_4_32B: &str = "glm-4-32b-0414-128k";
    
    // === GLM-4.6V Series (Vision/Multimodal models - use for computer vision) ===
    /// GLM-4.6V - Flagship vision model (128K context, supports video/image/text/file)
    pub const GLM_4_6V: &str = "glm-4.6v";
    /// GLM-4.6V-FlashX - Lightweight vision model, high-speed, affordable
    pub const GLM_4_6V_FLASHX: &str = "glm-4.6v-flashx";
    /// GLM-4.6V-Flash - Lightweight vision model, completely free
    pub const GLM_4_6V_FLASH: &str = "glm-4.6v-flash";
    
    // === GLM-4.5V Series (Previous generation vision models) ===
    pub const GLM_4_5V: &str = "glm-4.5v";
    
    // === Specialized Agent Models ===
    /// AutoGLM-Phone-Multilingual - For phone/device automation agents
    pub const AUTOGLM_PHONE: &str = "autoglm-phone-multilingual";
    
    // === Default model selections ===
    /// Default text model (GLM-4.7 for best performance)
    pub const DEFAULT_TEXT: &str = GLM_4_7;
    /// Default vision model (GLM-4.6V for computer vision tasks)
    pub const DEFAULT_VISION: &str = GLM_4_6V;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vision_model() {
        // Vision models should be detected
        assert!(GLMAdapter::is_vision_model("glm-4.6v"));
        assert!(GLMAdapter::is_vision_model("glm-4.6v-flashx"));
        assert!(GLMAdapter::is_vision_model("glm-4.6v-flash"));
        assert!(GLMAdapter::is_vision_model("glm-4.5v"));
        assert!(GLMAdapter::is_vision_model("GLM-4.6V"));
        assert!(GLMAdapter::is_vision_model("autoglm-phone-multilingual"));
        
        // Text models should NOT be detected as vision
        assert!(!GLMAdapter::is_vision_model("glm-4.7"));
        assert!(!GLMAdapter::is_vision_model("glm-4.7-flash"));
        assert!(!GLMAdapter::is_vision_model("glm-4.6"));
        assert!(!GLMAdapter::is_vision_model("glm-4.5"));
    }

    #[test]
    fn test_api_endpoint() {
        // All adapters should use the same API endpoint
        assert_eq!(GLM_API_BASE, "https://api.z.ai/api/paas/v4");
    }
    
    #[test]
    fn test_model_constants() {
        // Verify model names match official documentation
        assert_eq!(models::GLM_4_7, "glm-4.7");
        assert_eq!(models::GLM_4_7_FLASHX, "glm-4.7-flashx");
        assert_eq!(models::GLM_4_7_FLASH, "glm-4.7-flash");
        assert_eq!(models::GLM_4_6V, "glm-4.6v");
        assert_eq!(models::GLM_4_6V_FLASHX, "glm-4.6v-flashx");
        assert_eq!(models::DEFAULT_TEXT, "glm-4.7");
        assert_eq!(models::DEFAULT_VISION, "glm-4.6v");
    }
}
