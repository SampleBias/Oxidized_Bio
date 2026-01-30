//! Secure Password Exchange Protocol
//!
//! Implements RSA-based secure password exchange between local IDE and container.
//!
//! # Protocol Flow
//!
//! ```text
//! ┌─────────────────┐                    ┌─────────────────┐
//! │  Local Client   │                    │    Container    │
//! │   (IDE/CLI)     │                    │ (oxidized-bio)  │
//! └────────┬────────┘                    └────────┬────────┘
//!          │                                      │
//!          │ 1. Generate ephemeral RSA key pair  │
//!          │                                      │
//!          │ 2. POST /api/rfc/exchange           │
//!          │    { public_key_pem: "..." }        │
//!          │─────────────────────────────────────▶
//!          │                                      │
//!          │                    3. Get password from env
//!          │                    4. Encrypt with public key
//!          │                                      │
//!          │ 5. { encrypted_password: "..." }    │
//!          │◀─────────────────────────────────────
//!          │                                      │
//!          │ 6. Decrypt with private key         │
//!          │ 7. Discard private key              │
//!          │                                      │
//! ```
//!
//! # Security Properties
//!
//! - Private key never leaves the client
//! - Password is never transmitted in plaintext
//! - Each exchange uses a fresh key pair (forward secrecy)
//! - RSA-OAEP with SHA-256 padding

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use super::crypto;

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request for password exchange
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRequest {
    /// RSA public key in PEM format (hex-encoded)
    pub public_key_pem: String,
    
    /// Type of password to request
    #[serde(default)]
    pub password_type: PasswordType,
    
    /// Optional: client identifier for logging
    #[serde(default)]
    pub client_id: Option<String>,
}

/// Type of password to exchange
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PasswordType {
    /// RFC authentication password (default)
    #[default]
    Rfc,
    
    /// Root/admin password
    Root,
    
    /// SSH login password
    Ssh,
    
    /// Database password
    Database,
    
    /// Custom password by name
    Custom(String),
}

impl std::fmt::Display for PasswordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PasswordType::Rfc => write!(f, "rfc"),
            PasswordType::Root => write!(f, "root"),
            PasswordType::Ssh => write!(f, "ssh"),
            PasswordType::Database => write!(f, "database"),
            PasswordType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// Response containing encrypted password
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeResponse {
    /// Whether the exchange succeeded
    pub success: bool,
    
    /// Encrypted password (RSA-OAEP with SHA-256, hex-encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_password: Option<String>,
    
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Password type that was exchanged
    pub password_type: PasswordType,
    
    /// Timestamp of the exchange
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ExchangeResponse {
    /// Create a successful response
    fn success(encrypted_password: String, password_type: PasswordType) -> Self {
        Self {
            success: true,
            encrypted_password: Some(encrypted_password),
            error: None,
            password_type,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create an error response
    fn error(message: impl Into<String>, password_type: PasswordType) -> Self {
        Self {
            success: false,
            encrypted_password: None,
            error: Some(message.into()),
            password_type,
            timestamp: chrono::Utc::now(),
        }
    }
}

// =============================================================================
// Exchange Handler
// =============================================================================

/// Handle password exchange request
///
/// This endpoint does NOT require authentication since it's used to
/// bootstrap the authentication process itself.
pub async fn handle_exchange(Json(request): Json<ExchangeRequest>) -> impl IntoResponse {
    info!(
        "Password exchange requested: type={}, client={:?}",
        request.password_type,
        request.client_id
    );

    // Validate public key format
    let public_key = match crypto::import_public_key(&request.public_key_pem) {
        Ok(key) => key,
        Err(e) => {
            warn!("Invalid public key in exchange request: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(ExchangeResponse::error(
                    format!("Invalid public key: {}", e),
                    request.password_type,
                )),
            );
        }
    };

    // Get the appropriate password from environment
    let password = get_password_for_type(&request.password_type);

    if password.is_empty() {
        warn!(
            "Password not configured for type: {}",
            request.password_type
        );
        return (
            StatusCode::NOT_FOUND,
            Json(ExchangeResponse::error(
                format!("Password not configured for type: {}", request.password_type),
                request.password_type,
            )),
        );
    }

    // Encrypt the password with the provided public key
    match crypto::encrypt_data(&password, &public_key) {
        Ok(encrypted) => {
            info!(
                "Password exchange successful: type={}",
                request.password_type
            );
            (
                StatusCode::OK,
                Json(ExchangeResponse::success(encrypted, request.password_type)),
            )
        }
        Err(e) => {
            error!("Failed to encrypt password: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ExchangeResponse::error(
                    format!("Encryption failed: {}", e),
                    request.password_type,
                )),
            )
        }
    }
}

/// Get password from environment based on type
fn get_password_for_type(password_type: &PasswordType) -> String {
    match password_type {
        PasswordType::Rfc => std::env::var("RFC_PASSWORD").unwrap_or_default(),
        PasswordType::Root => std::env::var("ROOT_PASSWORD").unwrap_or_default(),
        PasswordType::Ssh => std::env::var("SSH_PASSWORD").unwrap_or_default(),
        PasswordType::Database => {
            // Try to extract from DATABASE_URL or use dedicated var
            std::env::var("DATABASE_PASSWORD")
                .or_else(|_| {
                    // Try to parse from DATABASE_URL
                    std::env::var("DATABASE_URL").ok().and_then(|url| {
                        // postgresql://user:password@host:port/db
                        url.split("://")
                            .nth(1)?
                            .split('@')
                            .next()?
                            .split(':')
                            .nth(1)
                            .map(|s| s.to_string())
                    }).ok_or(std::env::VarError::NotPresent)
                })
                .unwrap_or_default()
        }
        PasswordType::Custom(name) => {
            // Look for PASSWORD_<NAME> or <NAME>_PASSWORD
            let upper_name = name.to_uppercase();
            std::env::var(format!("PASSWORD_{}", upper_name))
                .or_else(|_| std::env::var(format!("{}_PASSWORD", upper_name)))
                .unwrap_or_default()
        }
    }
}

// =============================================================================
// Client-Side Helpers
// =============================================================================

/// Client-side utilities for password exchange
pub mod client {
    use super::*;

    /// Perform password exchange with a container
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the container API (e.g., "http://localhost:3000")
    /// * `password_type` - Type of password to request
    ///
    /// # Returns
    /// The decrypted password
    ///
    /// # Example
    /// ```ignore
    /// let password = exchange_password("http://localhost:3000", PasswordType::Rfc).await?;
    /// ```
    pub async fn exchange_password(
        base_url: &str,
        password_type: PasswordType,
    ) -> Result<String, ExchangeError> {
        // Generate ephemeral key pair
        let key_pair = crypto::generate_key_pair()
            .map_err(|e| ExchangeError::KeyGeneration(e.to_string()))?;
        
        let public_key_pem = crypto::export_public_key(&key_pair.public_key)
            .map_err(|e| ExchangeError::KeyGeneration(e.to_string()))?;

        // Build request
        let request = ExchangeRequest {
            public_key_pem,
            password_type: password_type.clone(),
            client_id: Some(format!("rust-client-{}", uuid::Uuid::new_v4())),
        };

        // Make HTTP request
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/rfc/exchange", base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(ExchangeError::ServerError(format!("{}: {}", status, text)));
        }

        let exchange_response: ExchangeResponse = response
            .json()
            .await
            .map_err(|e| ExchangeError::InvalidResponse(e.to_string()))?;

        if !exchange_response.success {
            return Err(ExchangeError::ServerError(
                exchange_response.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        // Decrypt the password
        let encrypted = exchange_response
            .encrypted_password
            .ok_or_else(|| ExchangeError::InvalidResponse("No encrypted password".to_string()))?;
        
        let password = crypto::decrypt_data(&encrypted, &key_pair.private_key)
            .map_err(|e| ExchangeError::Decryption(e.to_string()))?;

        // Private key is automatically dropped here
        Ok(password)
    }

    /// Errors that can occur during password exchange
    #[derive(Debug, thiserror::Error)]
    pub enum ExchangeError {
        #[error("Key generation failed: {0}")]
        KeyGeneration(String),

        #[error("Network error: {0}")]
        Network(String),

        #[error("Server error: {0}")]
        ServerError(String),

        #[error("Invalid response: {0}")]
        InvalidResponse(String),

        #[error("Decryption failed: {0}")]
        Decryption(String),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_type_display() {
        assert_eq!(PasswordType::Rfc.to_string(), "rfc");
        assert_eq!(PasswordType::Root.to_string(), "root");
        assert_eq!(PasswordType::Ssh.to_string(), "ssh");
        assert_eq!(PasswordType::Database.to_string(), "database");
        assert_eq!(
            PasswordType::Custom("api".to_string()).to_string(),
            "custom:api"
        );
    }

    #[test]
    fn test_password_type_default() {
        let default: PasswordType = Default::default();
        assert_eq!(default, PasswordType::Rfc);
    }

    #[test]
    fn test_exchange_response_success() {
        let response = ExchangeResponse::success("encrypted".to_string(), PasswordType::Rfc);
        assert!(response.success);
        assert_eq!(response.encrypted_password, Some("encrypted".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_exchange_response_error() {
        let response = ExchangeResponse::error("test error", PasswordType::Ssh);
        assert!(!response.success);
        assert!(response.encrypted_password.is_none());
        assert_eq!(response.error, Some("test error".to_string()));
    }

    #[test]
    fn test_get_password_for_type() {
        // Set test environment variables
        std::env::set_var("RFC_PASSWORD", "test_rfc");
        std::env::set_var("ROOT_PASSWORD", "test_root");
        
        assert_eq!(get_password_for_type(&PasswordType::Rfc), "test_rfc");
        assert_eq!(get_password_for_type(&PasswordType::Root), "test_root");
        
        // Clean up
        std::env::remove_var("RFC_PASSWORD");
        std::env::remove_var("ROOT_PASSWORD");
    }

    #[test]
    fn test_serialization() {
        let request = ExchangeRequest {
            public_key_pem: "abc123".to_string(),
            password_type: PasswordType::Rfc,
            client_id: Some("test-client".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: ExchangeRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.public_key_pem, "abc123");
        assert_eq!(parsed.password_type, PasswordType::Rfc);
        assert_eq!(parsed.client_id, Some("test-client".to_string()));
    }
}
