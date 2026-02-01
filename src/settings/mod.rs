//! Settings Module
//! 
//! Provides secure storage and retrieval of user settings including API keys.
//! API keys are encrypted at rest using AES-256-GCM.

pub mod storage;
pub mod routes;

pub use storage::*;
pub use routes::router;

use serde::{Deserialize, Serialize};

/// Available LLM providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    OpenAI,
    Anthropic,
    Google,
    OpenRouter,
    Groq,
}

impl Default for Provider {
    fn default() -> Self {
        Provider::OpenAI
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::OpenAI => write!(f, "openai"),
            Provider::Anthropic => write!(f, "anthropic"),
            Provider::Google => write!(f, "google"),
            Provider::OpenRouter => write!(f, "openrouter"),
            Provider::Groq => write!(f, "groq"),
        }
    }
}

impl Provider {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "openai" => Some(Provider::OpenAI),
            "anthropic" => Some(Provider::Anthropic),
            "google" => Some(Provider::Google),
            "openrouter" => Some(Provider::OpenRouter),
            "groq" => Some(Provider::Groq),
            _ => None,
        }
    }
}

/// API key configuration for a provider
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    /// The API key (encrypted at rest, decrypted when loaded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Default model for this provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
    /// Whether this provider is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Search API configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchApiConfig {
    /// SerpAPI key for Google Scholar and Google Light searches
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serpapi_key: Option<String>,
    /// Enable Google Scholar search (primary)
    #[serde(default = "default_true")]
    pub scholar_enabled: bool,
    /// Enable Google Light search (secondary)
    #[serde(default = "default_true")]
    pub light_enabled: bool,
}

/// User settings structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    /// Default LLM provider to use
    #[serde(default)]
    pub default_provider: Provider,
    
    /// OpenAI configuration
    #[serde(default)]
    pub openai: ProviderConfig,
    
    /// Anthropic configuration
    #[serde(default)]
    pub anthropic: ProviderConfig,
    
    /// Google configuration
    #[serde(default)]
    pub google: ProviderConfig,
    
    /// OpenRouter configuration
    #[serde(default)]
    pub openrouter: ProviderConfig,

    /// Groq configuration
    #[serde(default)]
    pub groq: ProviderConfig,
    
    /// Search API configuration (SerpAPI)
    #[serde(default)]
    pub search: SearchApiConfig,
    
    /// Theme preference
    #[serde(default)]
    pub theme: Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            default_provider: Provider::OpenAI,
            openai: ProviderConfig {
                api_key: None,
                default_model: Some("gpt-4o".to_string()),
                enabled: true,
            },
            anthropic: ProviderConfig {
                api_key: None,
                default_model: Some("claude-sonnet-4-20250514".to_string()),
                enabled: true,
            },
            google: ProviderConfig {
                api_key: None,
                default_model: Some("gemini-2.0-flash".to_string()),
                enabled: true,
            },
            openrouter: ProviderConfig {
                api_key: None,
                default_model: None,
                enabled: true,
            },
            groq: ProviderConfig {
                api_key: None,
                default_model: Some("groq/compound".to_string()),
                enabled: true,
            },
            search: SearchApiConfig {
                serpapi_key: None,
                scholar_enabled: true,
                light_enabled: true,
            },
            theme: Theme::Dark,
        }
    }
}

impl UserSettings {
    pub fn clear_api_keys(&mut self) {
        self.openai.api_key = None;
        self.anthropic.api_key = None;
        self.google.api_key = None;
        self.openrouter.api_key = None;
        self.groq.api_key = None;
    }

    pub fn set_single_provider_key(&mut self, provider_id: &str, key: String) {
        self.clear_api_keys();
        match provider_id {
            "openai" => self.openai.api_key = Some(key),
            "anthropic" => self.anthropic.api_key = Some(key),
            "google" => self.google.api_key = Some(key),
            "openrouter" => self.openrouter.api_key = Some(key),
            "groq" => self.groq.api_key = Some(key),
            _ => {}
        }

        if let Some(provider) = Provider::from_id(provider_id) {
            self.default_provider = provider;
        }
    }

    pub fn enforce_single_provider_key(&mut self) {
        let mut providers_with_keys = Vec::new();
        if self.openai.api_key.is_some() {
            providers_with_keys.push("openai");
        }
        if self.anthropic.api_key.is_some() {
            providers_with_keys.push("anthropic");
        }
        if self.google.api_key.is_some() {
            providers_with_keys.push("google");
        }
        if self.openrouter.api_key.is_some() {
            providers_with_keys.push("openrouter");
        }
        if self.groq.api_key.is_some() {
            providers_with_keys.push("groq");
        }

        if providers_with_keys.len() <= 1 {
            return;
        }

        let default_id = self.default_provider.to_string();
        let keep_id = if providers_with_keys.contains(&default_id.as_str()) {
            default_id
        } else {
            providers_with_keys[0].to_string()
        };

        let keep_key = match keep_id.as_str() {
            "openai" => self.openai.api_key.take(),
            "anthropic" => self.anthropic.api_key.take(),
            "google" => self.google.api_key.take(),
            "openrouter" => self.openrouter.api_key.take(),
            "groq" => self.groq.api_key.take(),
            _ => None,
        };

        self.clear_api_keys();
        if let Some(key) = keep_key {
            self.set_single_provider_key(&keep_id, key);
        }
    }
}

/// Settings response for the frontend (masks API keys)
#[derive(Debug, Clone, Serialize)]
pub struct SettingsResponse {
    pub default_provider: Provider,
    pub openai: ProviderStatus,
    pub anthropic: ProviderStatus,
    pub google: ProviderStatus,
    pub openrouter: ProviderStatus,
    pub groq: ProviderStatus,
    pub theme: Theme,
}

/// Provider status for frontend display (masks actual key)
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    /// Whether an API key is configured
    pub has_key: bool,
    /// Masked version of the key (last 4 chars only)
    pub key_hint: Option<String>,
    /// Default model for this provider
    pub default_model: Option<String>,
    /// Whether this provider is enabled
    pub enabled: bool,
}

impl From<&ProviderConfig> for ProviderStatus {
    fn from(config: &ProviderConfig) -> Self {
        let (has_key, key_hint) = match &config.api_key {
            Some(key) if key.len() > 4 => {
                let hint = format!("••••{}", &key[key.len()-4..]);
                (true, Some(hint))
            }
            Some(_) => (true, Some("••••".to_string())),
            None => (false, None),
        };
        
        Self {
            has_key,
            key_hint,
            default_model: config.default_model.clone(),
            enabled: config.enabled,
        }
    }
}

impl From<&UserSettings> for SettingsResponse {
    fn from(settings: &UserSettings) -> Self {
        Self {
            default_provider: settings.default_provider.clone(),
            openai: ProviderStatus::from(&settings.openai),
            anthropic: ProviderStatus::from(&settings.anthropic),
            google: ProviderStatus::from(&settings.google),
            openrouter: ProviderStatus::from(&settings.openrouter),
            groq: ProviderStatus::from(&settings.groq),
            theme: settings.theme.clone(),
        }
    }
}

/// Request to update settings
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSettingsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_provider: Option<Provider>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_model: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_model: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_model: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openrouter_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openrouter_model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub groq_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groq_model: Option<String>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<Theme>,
}
