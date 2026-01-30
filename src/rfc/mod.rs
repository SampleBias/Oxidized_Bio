//! Remote Function Call (RFC) System
//!
//! This module enables secure function invocation from external clients (IDE, CLI tools)
//! into the running Oxidized Bio container.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    LOCAL DEVELOPMENT INSTANCE                            │
//! │                                                                          │
//! │  call_rfc(func, *args, **kwargs)                                        │
//! │              │                                                           │
//! │              ▼                                                           │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │  1. Serialize: module, function_name, args, kwargs               │   │
//! │  │  2. Create hash: HMAC-SHA256(serialized_data, RFC_PASSWORD)      │   │
//! │  │  3. Send HTTP POST to container /api/rfc endpoint                │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────────────┘
//!                               │
//!                               │ HTTP POST (JSON)
//!                               ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                    DOCKER CONTAINER INSTANCE                             │
//! │                                                                          │
//! │  /api/rfc endpoint                                                       │
//! │              │                                                           │
//! │              ▼                                                           │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │  1. Verify hash with RFC_PASSWORD                                │   │
//! │  │  2. If valid: route to appropriate handler                       │   │
//! │  │  3. Execute function and return result as JSON                   │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Endpoints
//!
//! - `POST /api/rfc` - Execute an RFC call (requires HMAC auth)
//! - `POST /api/rfc/exchange` - Password exchange (no auth, uses RSA)
//! - `GET /api/rfc/health` - Health check (no auth)
//! - `POST /api/rfc/shell` - Execute shell command (requires HMAC auth)

pub mod crypto;
pub mod exchange;
pub mod shell;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::models::AppState;
use shell::{CommandRequest, CommandResult};

// =============================================================================
// RFC Input/Output Types
// =============================================================================

/// RFC input structure - describes the function to call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFCInput {
    /// Module path (e.g., "system", "shell", "health")
    pub module: String,
    
    /// Function name to call
    pub function_name: String,
    
    /// Positional arguments (JSON serialized)
    #[serde(default)]
    pub args: Vec<serde_json::Value>,
    
    /// Keyword arguments (JSON serialized)
    #[serde(default)]
    pub kwargs: HashMap<String, serde_json::Value>,
}

/// RFC call wrapper with authentication hash
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFCCall {
    /// JSON-serialized RFCInput
    pub rfc_input: String,
    
    /// HMAC-SHA256 hash of rfc_input using RFC_PASSWORD
    pub hash: String,
}

/// RFC response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFCResponse {
    /// Whether the call succeeded
    pub success: bool,
    
    /// Result data (if success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl RFCResponse {
    /// Create a successful response
    fn success(result: serde_json::Value, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            result: Some(result),
            error: None,
            execution_time_ms,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create an error response
    fn error(message: impl Into<String>, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            result: None,
            error: Some(message.into()),
            execution_time_ms,
            timestamp: chrono::Utc::now(),
        }
    }
}

// =============================================================================
// RFC Router
// =============================================================================

/// Create RFC router with all endpoints
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/rfc", post(handle_rfc))
        .route("/api/rfc/exchange", post(exchange::handle_exchange))
        .route("/api/rfc/health", get(handle_rfc_health))
        .route("/api/rfc/shell", post(handle_shell))
        .route("/api/rfc/sessions", get(handle_list_sessions))
        .with_state(state)
}

// =============================================================================
// Endpoint Handlers
// =============================================================================

/// Main RFC handler - processes authenticated function calls
async fn handle_rfc(
    State(state): State<AppState>,
    Json(call): Json<RFCCall>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Get RFC password from environment
    let rfc_password = std::env::var("RFC_PASSWORD").unwrap_or_default();
    
    if rfc_password.is_empty() {
        warn!("RFC_PASSWORD not set - RFC calls disabled");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(RFCResponse::error(
                "RFC not configured (RFC_PASSWORD not set)",
                start.elapsed().as_millis() as u64,
            )),
        );
    }

    // Verify HMAC signature
    if !crypto::verify_data(&call.rfc_input, &call.hash, &rfc_password) {
        warn!("RFC call rejected: invalid HMAC hash");
        return (
            StatusCode::UNAUTHORIZED,
            Json(RFCResponse::error(
                "Invalid RFC authentication hash",
                start.elapsed().as_millis() as u64,
            )),
        );
    }

    // Parse the RFCInput
    let input: RFCInput = match serde_json::from_str(&call.rfc_input) {
        Ok(input) => input,
        Err(e) => {
            error!("RFC input parse error: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(RFCResponse::error(
                    format!("Invalid RFC input: {}", e),
                    start.elapsed().as_millis() as u64,
                )),
            );
        }
    };

    info!("RFC call: {}.{}", input.module, input.function_name);

    // Execute the function based on module and function_name
    let result = execute_rfc_function(&state, &input).await;

    match result {
        Ok(value) => (
            StatusCode::OK,
            Json(RFCResponse::success(value, start.elapsed().as_millis() as u64)),
        ),
        Err(e) => {
            error!("RFC execution error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RFCResponse::error(e.to_string(), start.elapsed().as_millis() as u64)),
            )
        }
    }
}

/// Execute RFC function based on module path and function name
async fn execute_rfc_function(
    state: &AppState,
    input: &RFCInput,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    match (input.module.as_str(), input.function_name.as_str()) {
        // ======================
        // System Functions
        // ======================
        ("system", "ping") => Ok(serde_json::json!({
            "pong": true,
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),

        ("system", "info") => Ok(serde_json::json!({
            "name": "oxidized-bio",
            "version": env!("CARGO_PKG_VERSION"),
            "rust_version": env!("CARGO_PKG_RUST_VERSION"),
            "container_id": std::env::var("CONTAINER_ID").ok(),
            "hostname": hostname::get().ok().map(|h| h.to_string_lossy().to_string()),
            "uptime_seconds": get_uptime_seconds(),
        })),

        ("system", "env") => {
            // Return safe environment info (not secrets)
            let safe_keys = vec![
                "RUST_LOG", "APP_ENV", "PORT", "HOST", "TZ",
                "RFC_ENABLED", "ENABLE_SSH", "AUTH_MODE",
            ];
            let env_vars: HashMap<String, String> = safe_keys
                .iter()
                .filter_map(|k| std::env::var(k).ok().map(|v| (k.to_string(), v)))
                .collect();
            Ok(serde_json::to_value(env_vars)?)
        }

        // ======================
        // Health Functions
        // ======================
        ("health", "check") => Ok(serde_json::json!({
            "status": "healthy",
            "database": !state.pool.is_closed(),
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),

        ("health", "detailed") => {
            let db_status = if state.pool.is_closed() {
                "disconnected"
            } else {
                "connected"
            };
            
            Ok(serde_json::json!({
                "status": "healthy",
                "components": {
                    "database": db_status,
                    "rfc": "enabled",
                    "ssh": std::env::var("ENABLE_SSH").unwrap_or_else(|_| "true".to_string()),
                },
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }

        // ======================
        // Shell Functions
        // ======================
        ("shell", "execute") => {
            let command = input.args.get(0)
                .and_then(|v| v.as_str())
                .ok_or("Missing command argument")?;
            
            let cwd = input.kwargs.get("cwd")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            
            let timeout_ms = input.kwargs.get("timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(30000);
            
            let result = execute_shell_command(command, cwd, timeout_ms).await?;
            Ok(serde_json::to_value(result)?)
        }

        ("shell", "cwd") => {
            let cwd = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/app".to_string());
            Ok(serde_json::json!({ "cwd": cwd }))
        }

        // ======================
        // File Functions
        // ======================
        ("file", "read") => {
            let path = input.args.get(0)
                .and_then(|v| v.as_str())
                .ok_or("Missing path argument")?;
            
            // Security: only allow reading from /app directory
            let safe_path = std::path::Path::new(path);
            if !safe_path.starts_with("/app") && !safe_path.is_relative() {
                return Err("Access denied: can only read from /app".into());
            }
            
            let content = tokio::fs::read_to_string(path).await?;
            Ok(serde_json::json!({ "content": content }))
        }

        ("file", "exists") => {
            let path = input.args.get(0)
                .and_then(|v| v.as_str())
                .ok_or("Missing path argument")?;
            
            let exists = tokio::fs::metadata(path).await.is_ok();
            Ok(serde_json::json!({ "exists": exists }))
        }

        ("file", "list") => {
            let path = input.args.get(0)
                .and_then(|v| v.as_str())
                .unwrap_or("/app");
            
            let mut entries = Vec::new();
            let mut dir = tokio::fs::read_dir(path).await?;
            
            while let Some(entry) = dir.next_entry().await? {
                let metadata = entry.metadata().await?;
                entries.push(serde_json::json!({
                    "name": entry.file_name().to_string_lossy(),
                    "is_dir": metadata.is_dir(),
                    "size": metadata.len(),
                }));
            }
            
            Ok(serde_json::json!({ "entries": entries }))
        }

        // ======================
        // Unknown Function
        // ======================
        _ => Err(format!(
            "Unknown RFC function: {}.{}",
            input.module, input.function_name
        ).into()),
    }
}

/// Execute a shell command
async fn execute_shell_command(
    command: &str,
    cwd: Option<String>,
    timeout_ms: u64,
) -> Result<CommandResult, Box<dyn std::error::Error + Send + Sync>> {
    use tokio::process::Command;
    use tokio::time::{timeout, Duration};

    let start = std::time::Instant::now();

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);

    if let Some(dir) = &cwd {
        cmd.current_dir(dir);
    }

    let output = timeout(
        Duration::from_millis(timeout_ms),
        cmd.output()
    ).await;

    match output {
        Ok(Ok(output)) => Ok(CommandResult {
            session_id: uuid::Uuid::new_v4(),
            command: command.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms: start.elapsed().as_millis() as u64,
            timed_out: false,
            completed_at: chrono::Utc::now(),
        }),
        Ok(Err(e)) => Err(format!("Command execution failed: {}", e).into()),
        Err(_) => Ok(CommandResult {
            session_id: uuid::Uuid::new_v4(),
            command: command.to_string(),
            stdout: String::new(),
            stderr: format!("Command timed out after {}ms", timeout_ms),
            exit_code: -1,
            duration_ms: timeout_ms,
            timed_out: true,
            completed_at: chrono::Utc::now(),
        }),
    }
}

/// Handle direct shell command execution endpoint
async fn handle_shell(
    State(_state): State<AppState>,
    Json(call): Json<RFCCall>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();

    // Verify authentication
    let rfc_password = std::env::var("RFC_PASSWORD").unwrap_or_default();
    
    if !crypto::verify_data(&call.rfc_input, &call.hash, &rfc_password) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(RFCResponse::error(
                "Invalid authentication",
                start.elapsed().as_millis() as u64,
            )),
        );
    }

    // Parse command request
    let request: CommandRequest = match serde_json::from_str(&call.rfc_input) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(RFCResponse::error(
                    format!("Invalid request: {}", e),
                    start.elapsed().as_millis() as u64,
                )),
            );
        }
    };

    // Execute command
    match execute_shell_command(&request.command, request.cwd, request.timeout_ms).await {
        Ok(result) => (
            StatusCode::OK,
            Json(RFCResponse::success(
                serde_json::to_value(result).unwrap(),
                start.elapsed().as_millis() as u64,
            )),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RFCResponse::error(e.to_string(), start.elapsed().as_millis() as u64)),
        ),
    }
}

/// RFC health check (no auth required)
async fn handle_rfc_health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "rfc_enabled": std::env::var("RFC_ENABLED").unwrap_or_else(|_| "true".to_string()) == "true",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// List active SSH sessions
async fn handle_list_sessions(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // For now, return empty - would integrate with SSHSessionManager in production
    Json(serde_json::json!({
        "sessions": [],
        "count": 0
    }))
}

/// Get system uptime in seconds
fn get_uptime_seconds() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|s| s.split_whitespace().next().map(|s| s.to_string()))
            .and_then(|s| s.parse::<f64>().ok())
            .map(|f| f as u64)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

// =============================================================================
// Client Helpers
// =============================================================================

/// Client-side helper for making RFC calls
pub mod client {
    use super::*;

    /// Make an RFC call to a container
    pub async fn call_rfc(
        base_url: &str,
        password: &str,
        module: &str,
        function_name: &str,
        args: Vec<serde_json::Value>,
        kwargs: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, RFCError> {
        let input = RFCInput {
            module: module.to_string(),
            function_name: function_name.to_string(),
            args,
            kwargs,
        };

        let rfc_input = serde_json::to_string(&input)
            .map_err(|e| RFCError::Serialization(e.to_string()))?;

        let hash = crypto::hash_data(&rfc_input, password);

        let call = RFCCall { rfc_input, hash };

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/rfc", base_url))
            .json(&call)
            .send()
            .await
            .map_err(|e| RFCError::Network(e.to_string()))?;

        let rfc_response: RFCResponse = response
            .json()
            .await
            .map_err(|e| RFCError::InvalidResponse(e.to_string()))?;

        if rfc_response.success {
            rfc_response.result.ok_or_else(|| RFCError::NoResult)
        } else {
            Err(RFCError::ServerError(
                rfc_response.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    /// RFC client errors
    #[derive(Debug, thiserror::Error)]
    pub enum RFCError {
        #[error("Serialization error: {0}")]
        Serialization(String),

        #[error("Network error: {0}")]
        Network(String),

        #[error("Invalid response: {0}")]
        InvalidResponse(String),

        #[error("Server error: {0}")]
        ServerError(String),

        #[error("No result returned")]
        NoResult,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfc_input_serialization() {
        let input = RFCInput {
            module: "system".to_string(),
            function_name: "ping".to_string(),
            args: vec![],
            kwargs: HashMap::new(),
        };

        let json = serde_json::to_string(&input).unwrap();
        let parsed: RFCInput = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.module, "system");
        assert_eq!(parsed.function_name, "ping");
    }

    #[test]
    fn test_rfc_call_hash_verification() {
        let input = RFCInput {
            module: "test".to_string(),
            function_name: "func".to_string(),
            args: vec![],
            kwargs: HashMap::new(),
        };

        let rfc_input = serde_json::to_string(&input).unwrap();
        let password = "test_password";
        let hash = crypto::hash_data(&rfc_input, password);

        assert!(crypto::verify_data(&rfc_input, &hash, password));
        assert!(!crypto::verify_data(&rfc_input, &hash, "wrong_password"));
    }

    #[test]
    fn test_rfc_response_success() {
        let response = RFCResponse::success(serde_json::json!({"test": true}), 100);
        assert!(response.success);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_rfc_response_error() {
        let response = RFCResponse::error("test error", 50);
        assert!(!response.success);
        assert!(response.result.is_none());
        assert_eq!(response.error, Some("test error".to_string()));
    }
}
