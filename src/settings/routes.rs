//! Settings API Routes
//! 
//! Provides REST endpoints for managing user settings:
//! - GET /api/settings - Get current settings (with masked API keys)
//! - POST /api/settings - Update settings
//! - GET /api/settings/providers - List available providers

use axum::{
    Router,
    routing::{get, post},
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use super::{
    SettingsStorage, UserSettings, SettingsResponse, UpdateSettingsRequest,
    Provider, ProviderStatus,
};
use serde::Serialize;
use tracing::{info, error};

/// Create the settings router
pub fn router() -> Router {
    Router::new()
        .route("/api/settings", get(get_settings))
        .route("/api/settings", post(update_settings))
        .route("/api/settings/providers", get(list_providers))
        .route("/api/settings/test/{provider}", post(test_provider))
}

/// GET /api/settings - Get current settings
async fn get_settings() -> impl IntoResponse {
    let storage = SettingsStorage::new();
    
    match storage.load().await {
        Ok(settings) => {
            let response = SettingsResponse::from(&settings);
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Failed to load settings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to load settings",
                    "details": e.to_string()
                }))
            ).into_response()
        }
    }
}

/// POST /api/settings - Update settings
async fn update_settings(
    Json(request): Json<UpdateSettingsRequest>,
) -> impl IntoResponse {
    let storage = SettingsStorage::new();
    
    // Load existing settings
    let mut settings = match storage.load().await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to load settings: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to load existing settings",
                    "details": e.to_string()
                }))
            ).into_response();
        }
    };

    // Apply updates
    if let Some(provider) = request.default_provider {
        settings.default_provider = provider;
    }
    
    // Update OpenAI
    if let Some(key) = request.openai_key {
        if key.is_empty() {
            settings.openai.api_key = None;
        } else {
            settings.openai.api_key = Some(key);
        }
    }
    if let Some(model) = request.openai_model {
        settings.openai.default_model = Some(model);
    }
    
    // Update Anthropic
    if let Some(key) = request.anthropic_key {
        if key.is_empty() {
            settings.anthropic.api_key = None;
        } else {
            settings.anthropic.api_key = Some(key);
        }
    }
    if let Some(model) = request.anthropic_model {
        settings.anthropic.default_model = Some(model);
    }
    
    // Update Google
    if let Some(key) = request.google_key {
        if key.is_empty() {
            settings.google.api_key = None;
        } else {
            settings.google.api_key = Some(key);
        }
    }
    if let Some(model) = request.google_model {
        settings.google.default_model = Some(model);
    }
    
    // Update OpenRouter
    if let Some(key) = request.openrouter_key {
        if key.is_empty() {
            settings.openrouter.api_key = None;
        } else {
            settings.openrouter.api_key = Some(key);
        }
    }
    if let Some(model) = request.openrouter_model {
        settings.openrouter.default_model = Some(model);
    }
    
    // Update GLM
    if let Some(key) = request.glm_key {
        if key.is_empty() {
            settings.glm.api_key = None;
        } else {
            settings.glm.api_key = Some(key);
        }
    }
    if let Some(model) = request.glm_model {
        settings.glm.default_model = Some(model);
    }
    
    // Update theme
    if let Some(theme) = request.theme {
        settings.theme = theme;
    }

    // Save settings
    match storage.save(&settings).await {
        Ok(_) => {
            info!("Settings updated successfully");
            let response = SettingsResponse::from(&settings);
            (StatusCode::OK, Json(serde_json::json!({
                "success": true,
                "message": "Settings saved successfully",
                "settings": response
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to save settings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to save settings",
                    "details": e.to_string()
                }))
            ).into_response()
        }
    }
}

/// Provider information for the frontend
#[derive(Serialize)]
struct ProviderInfo {
    id: String,
    name: String,
    description: String,
    models: Vec<ModelInfo>,
    docs_url: Option<String>,
}

#[derive(Serialize)]
struct ModelInfo {
    id: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    supports_vision: Option<bool>,
}

/// GET /api/settings/providers - List available providers
async fn list_providers() -> impl IntoResponse {
    let providers = vec![
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            description: "GPT-4o, GPT-4, and other OpenAI models".to_string(),
            models: vec![
                ModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string(), context_length: Some(128000), supports_vision: Some(true) },
                ModelInfo { id: "gpt-4o-mini".to_string(), name: "GPT-4o Mini".to_string(), context_length: Some(128000), supports_vision: Some(true) },
                ModelInfo { id: "gpt-4-turbo".to_string(), name: "GPT-4 Turbo".to_string(), context_length: Some(128000), supports_vision: Some(true) },
                ModelInfo { id: "o1".to_string(), name: "o1".to_string(), context_length: Some(200000), supports_vision: Some(false) },
                ModelInfo { id: "o1-mini".to_string(), name: "o1-mini".to_string(), context_length: Some(128000), supports_vision: Some(false) },
            ],
            docs_url: Some("https://platform.openai.com/docs".to_string()),
        },
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            description: "Claude 4, Claude 3.5, and Claude 3 models".to_string(),
            models: vec![
                ModelInfo { id: "claude-sonnet-4-20250514".to_string(), name: "Claude Sonnet 4".to_string(), context_length: Some(200000), supports_vision: Some(true) },
                ModelInfo { id: "claude-3-5-sonnet-20241022".to_string(), name: "Claude 3.5 Sonnet".to_string(), context_length: Some(200000), supports_vision: Some(true) },
                ModelInfo { id: "claude-3-opus-20240229".to_string(), name: "Claude 3 Opus".to_string(), context_length: Some(200000), supports_vision: Some(true) },
                ModelInfo { id: "claude-3-haiku-20240307".to_string(), name: "Claude 3 Haiku".to_string(), context_length: Some(200000), supports_vision: Some(true) },
            ],
            docs_url: Some("https://docs.anthropic.com".to_string()),
        },
        ProviderInfo {
            id: "google".to_string(),
            name: "Google AI".to_string(),
            description: "Gemini 2.0, Gemini 1.5 models".to_string(),
            models: vec![
                ModelInfo { id: "gemini-2.0-flash".to_string(), name: "Gemini 2.0 Flash".to_string(), context_length: Some(1000000), supports_vision: Some(true) },
                ModelInfo { id: "gemini-2.0-flash-thinking".to_string(), name: "Gemini 2.0 Flash Thinking".to_string(), context_length: Some(1000000), supports_vision: Some(true) },
                ModelInfo { id: "gemini-1.5-pro".to_string(), name: "Gemini 1.5 Pro".to_string(), context_length: Some(2000000), supports_vision: Some(true) },
                ModelInfo { id: "gemini-1.5-flash".to_string(), name: "Gemini 1.5 Flash".to_string(), context_length: Some(1000000), supports_vision: Some(true) },
            ],
            docs_url: Some("https://ai.google.dev/docs".to_string()),
        },
        ProviderInfo {
            id: "glm".to_string(),
            name: "GLM (Zhipu AI)".to_string(),
            description: "GLM-4.7 for coding, GLM-4.6/4.5 for general use".to_string(),
            models: vec![
                ModelInfo { id: "glm-4.7".to_string(), name: "GLM-4.7 (Coding)".to_string(), context_length: Some(128000), supports_vision: Some(false) },
                ModelInfo { id: "glm-4.6".to_string(), name: "GLM-4.6".to_string(), context_length: Some(128000), supports_vision: Some(false) },
                ModelInfo { id: "glm-4.6v".to_string(), name: "GLM-4.6V (Vision)".to_string(), context_length: Some(128000), supports_vision: Some(true) },
                ModelInfo { id: "glm-4.5".to_string(), name: "GLM-4.5".to_string(), context_length: Some(128000), supports_vision: Some(false) },
                ModelInfo { id: "glm-4.5v".to_string(), name: "GLM-4.5V (Vision)".to_string(), context_length: Some(128000), supports_vision: Some(true) },
            ],
            docs_url: Some("https://docs.z.ai".to_string()),
        },
        ProviderInfo {
            id: "openrouter".to_string(),
            name: "OpenRouter".to_string(),
            description: "Access multiple providers through a single API".to_string(),
            models: vec![
                ModelInfo { id: "anthropic/claude-sonnet-4".to_string(), name: "Claude Sonnet 4 (via OpenRouter)".to_string(), context_length: None, supports_vision: None },
                ModelInfo { id: "openai/gpt-4o".to_string(), name: "GPT-4o (via OpenRouter)".to_string(), context_length: None, supports_vision: None },
                ModelInfo { id: "google/gemini-2.0-flash".to_string(), name: "Gemini 2.0 Flash (via OpenRouter)".to_string(), context_length: None, supports_vision: None },
            ],
            docs_url: Some("https://openrouter.ai/docs".to_string()),
        },
    ];

    Json(providers)
}

/// POST /api/settings/test/{provider} - Test provider connection
async fn test_provider(
    axum::extract::Path(provider): axum::extract::Path<String>,
) -> impl IntoResponse {
    let storage = SettingsStorage::new();
    
    // Get the API key for the provider
    let api_key = match storage.get_api_key(&provider).await {
        Ok(Some(key)) => key,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "error": "No API key configured for this provider"
                }))
            ).into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to load settings: {}", e)
                }))
            ).into_response();
        }
    };

    // Test the connection based on provider
    let test_result = match provider.to_lowercase().as_str() {
        "openai" => test_openai(&api_key).await,
        "anthropic" => test_anthropic(&api_key).await,
        "google" => test_google(&api_key).await,
        "glm" => test_glm(&api_key).await,
        "openrouter" => test_openrouter(&api_key).await,
        _ => Err(format!("Unknown provider: {}", provider)),
    };

    match test_result {
        Ok(message) => {
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "message": message
                }))
            ).into_response()
        }
        Err(error) => {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "error": error
                }))
            ).into_response()
        }
    }
}

// Provider test functions
async fn test_openai(api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if response.status().is_success() {
        Ok("OpenAI API key is valid".to_string())
    } else {
        Err(format!("API returned error: {}", response.status()))
    }
}

async fn test_anthropic(api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(r#"{"model":"claude-3-haiku-20240307","max_tokens":1,"messages":[{"role":"user","content":"Hi"}]}"#)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if response.status().is_success() || response.status().as_u16() == 400 {
        // 400 might be returned for invalid request, but key is valid
        Ok("Anthropic API key is valid".to_string())
    } else if response.status().as_u16() == 401 {
        Err("Invalid API key".to_string())
    } else {
        Err(format!("API returned error: {}", response.status()))
    }
}

async fn test_google(api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models?key={}",
        api_key
    );
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if response.status().is_success() {
        Ok("Google AI API key is valid".to_string())
    } else {
        Err(format!("API returned error: {}", response.status()))
    }
}

async fn test_glm(api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.z.ai/api/paas/v4/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .body(r#"{"model":"glm-4.5","messages":[{"role":"user","content":"Hi"}],"max_tokens":1}"#)
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if response.status().is_success() {
        Ok("GLM API key is valid".to_string())
    } else if response.status().as_u16() == 401 {
        Err("Invalid API key".to_string())
    } else {
        Err(format!("API returned error: {}", response.status()))
    }
}

async fn test_openrouter(api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://openrouter.ai/api/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    if response.status().is_success() {
        Ok("OpenRouter API key is valid".to_string())
    } else {
        Err(format!("API returned error: {}", response.status()))
    }
}
