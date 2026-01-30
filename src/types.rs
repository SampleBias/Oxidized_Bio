// Type definitions and enums

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    Google,
    OpenRouter,
    GLM,
}

impl std::fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProvider::OpenAI => write!(f, "openai"),
            LLMProvider::Anthropic => write!(f, "anthropic"),
            LLMProvider::Google => write!(f, "google"),
            LLMProvider::OpenRouter => write!(f, "openrouter"),
            LLMProvider::GLM => write!(f, "glm"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMRequest {
    pub provider: String,
    pub model: String,
    pub messages: Vec<LLMMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system_instruction: Option<String>,
}

/// Content part for multimodal messages (text, images, etc.)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>, // "low", "high", or "auto"
    },
    #[serde(rename = "image_base64")]
    ImageBase64 {
        base64: String,
        media_type: String, // e.g., "image/jpeg", "image/png"
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
    },
}

/// Message content - can be simple text or multimodal (text + images)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Multimodal(Vec<ContentPart>),
}

impl MessageContent {
    /// Create a simple text content
    pub fn text(s: impl Into<String>) -> Self {
        MessageContent::Text(s.into())
    }

    /// Create multimodal content with text and images
    pub fn multimodal(parts: Vec<ContentPart>) -> Self {
        MessageContent::Multimodal(parts)
    }

    /// Get the text content (for simple text or first text part in multimodal)
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(s) => Some(s),
            MessageContent::Multimodal(parts) => {
                parts.iter().find_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
            }
        }
    }

    /// Check if this content contains images
    pub fn has_images(&self) -> bool {
        match self {
            MessageContent::Text(_) => false,
            MessageContent::Multimodal(parts) => {
                parts.iter().any(|p| matches!(p, ContentPart::ImageUrl { .. } | ContentPart::ImageBase64 { .. }))
            }
        }
    }
}

impl From<String> for MessageContent {
    fn from(s: String) -> Self {
        MessageContent::Text(s)
    }
}

impl From<&str> for MessageContent {
    fn from(s: &str) -> Self {
        MessageContent::Text(s.to_string())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMMessage {
    pub role: String, // "user", "assistant", "system"
    pub content: MessageContent,
}

impl LLMMessage {
    /// Create a new message with text content
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a new message with multimodal content
    pub fn with_content(role: impl Into<String>, content: MessageContent) -> Self {
        Self {
            role: role.into(),
            content,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }

    /// Create a user message with image (for vision models)
    pub fn user_with_image(text: impl Into<String>, image_url: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: MessageContent::Multimodal(vec![
                ContentPart::Text { text: text.into() },
                ContentPart::ImageUrl { url: image_url.into(), detail: None },
            ]),
        }
    }

    /// Create a user message with base64 image
    pub fn user_with_base64_image(
        text: impl Into<String>,
        base64: impl Into<String>,
        media_type: impl Into<String>,
    ) -> Self {
        Self {
            role: "user".to_string(),
            content: MessageContent::Multimodal(vec![
                ContentPart::Text { text: text.into() },
                ContentPart::ImageBase64 {
                    base64: base64.into(),
                    media_type: media_type.into(),
                    detail: None,
                },
            ]),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LLMResponse {
    pub content: String,
    pub finish_reason: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("LLM API error: {0}")]
    LLMApi(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type AppResult<T> = std::result::Result<T, AppError>;
