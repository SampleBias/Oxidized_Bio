//! SSH Session Management Types
//!
//! This module provides types for tracking SSH connections and managing
//! command execution in the containerized environment.
//!
//! # Architecture
//! ```text
//! ┌─────────────────┐       ┌─────────────────┐
//! │  Local Client   │──SSH──│  Container      │
//! │  (IDE/CLI)      │       │  (oxidized-bio) │
//! └─────────────────┘       └─────────────────┘
//!         │                         │
//!         │    CommandRequest       │
//!         │─────────────────────────▶
//!         │                         │
//!         │    CommandResult        │
//!         │◀─────────────────────────
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// =============================================================================
// SSH Session Types
// =============================================================================

/// Represents an active SSH session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSHSession {
    /// Unique session identifier
    pub id: Uuid,
    
    /// Username of the connected user
    pub user: String,
    
    /// Timestamp when the session was established
    pub connected_at: DateTime<Utc>,
    
    /// Timestamp of the last activity
    pub last_activity: DateTime<Utc>,
    
    /// Current working directory
    pub cwd: String,
    
    /// Process ID of the shell (if available)
    pub pid: Option<u32>,
    
    /// Client IP address
    pub client_ip: Option<String>,
    
    /// Session state
    pub state: SessionState,
}

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Session is active and ready
    Active,
    /// Session is executing a command
    Busy,
    /// Session is being closed
    Closing,
    /// Session has been closed
    Closed,
}

impl Default for SessionState {
    fn default() -> Self {
        SessionState::Active
    }
}

impl SSHSession {
    /// Create a new SSH session
    pub fn new(user: String, cwd: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user,
            connected_at: now,
            last_activity: now,
            cwd,
            pid: None,
            client_ip: None,
            state: SessionState::Active,
        }
    }

    /// Update the last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Check if the session is idle (no activity for given duration)
    pub fn is_idle(&self, max_idle_seconds: i64) -> bool {
        let idle_duration = Utc::now() - self.last_activity;
        idle_duration.num_seconds() > max_idle_seconds
    }

    /// Get session duration in seconds
    pub fn duration_seconds(&self) -> i64 {
        (Utc::now() - self.connected_at).num_seconds()
    }
}

// =============================================================================
// Command Execution Types
// =============================================================================

/// Request to execute a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRequest {
    /// Session ID (optional - creates new session if not provided)
    #[serde(default)]
    pub session_id: Option<Uuid>,
    
    /// Command to execute
    pub command: String,
    
    /// Working directory (overrides session cwd)
    #[serde(default)]
    pub cwd: Option<String>,
    
    /// Timeout in milliseconds (default: 30000)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    
    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>,
    
    /// Whether to capture stdout
    #[serde(default = "default_true")]
    pub capture_stdout: bool,
    
    /// Whether to capture stderr
    #[serde(default = "default_true")]
    pub capture_stderr: bool,
}

fn default_timeout() -> u64 {
    30000 // 30 seconds
}

fn default_true() -> bool {
    true
}

impl CommandRequest {
    /// Create a simple command request
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            session_id: None,
            command: command.into(),
            cwd: None,
            timeout_ms: default_timeout(),
            env: HashMap::new(),
            capture_stdout: true,
            capture_stderr: true,
        }
    }

    /// Set the working directory
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

/// Result of command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// Session ID that executed the command
    pub session_id: Uuid,
    
    /// The command that was executed
    pub command: String,
    
    /// Standard output
    pub stdout: String,
    
    /// Standard error
    pub stderr: String,
    
    /// Exit code (0 = success)
    pub exit_code: i32,
    
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    
    /// Whether the command timed out
    pub timed_out: bool,
    
    /// Timestamp when execution completed
    pub completed_at: DateTime<Utc>,
}

impl CommandResult {
    /// Check if the command succeeded (exit code 0)
    pub fn success(&self) -> bool {
        self.exit_code == 0 && !self.timed_out
    }

    /// Get combined output (stdout + stderr)
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

// =============================================================================
// Session Manager
// =============================================================================

/// Manager for tracking active SSH sessions
#[derive(Clone)]
pub struct SSHSessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, SSHSession>>>,
    max_sessions: usize,
    idle_timeout_seconds: i64,
}

impl SSHSessionManager {
    /// Create a new session manager
    pub fn new(max_sessions: usize, idle_timeout_seconds: i64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions,
            idle_timeout_seconds,
        }
    }

    /// Register a new session
    pub async fn register_session(&self, session: SSHSession) -> Result<Uuid, SessionError> {
        let mut sessions = self.sessions.write().await;
        
        // Check if we've reached the limit
        if sessions.len() >= self.max_sessions {
            // Try to clean up idle sessions first
            let idle_ids: Vec<Uuid> = sessions
                .iter()
                .filter(|(_, s)| s.is_idle(self.idle_timeout_seconds))
                .map(|(id, _)| *id)
                .collect();
            
            for id in idle_ids {
                sessions.remove(&id);
            }
            
            // Check again
            if sessions.len() >= self.max_sessions {
                return Err(SessionError::MaxSessionsReached);
            }
        }
        
        let id = session.id;
        sessions.insert(id, session);
        Ok(id)
    }

    /// Remove a session
    pub async fn remove_session(&self, id: &Uuid) -> Option<SSHSession> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(id)
    }

    /// Get a session by ID
    pub async fn get_session(&self, id: &Uuid) -> Option<SSHSession> {
        let sessions = self.sessions.read().await;
        sessions.get(id).cloned()
    }

    /// List all active sessions
    pub async fn list_sessions(&self) -> Vec<SSHSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Update the last activity timestamp for a session
    pub async fn update_activity(&self, id: &Uuid) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(id) {
            session.touch();
            true
        } else {
            false
        }
    }

    /// Update session state
    pub async fn update_state(&self, id: &Uuid, state: SessionState) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(id) {
            session.state = state;
            true
        } else {
            false
        }
    }

    /// Clean up idle sessions
    pub async fn cleanup_idle_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let idle_ids: Vec<Uuid> = sessions
            .iter()
            .filter(|(_, s)| s.is_idle(self.idle_timeout_seconds))
            .map(|(id, _)| *id)
            .collect();
        
        let count = idle_ids.len();
        for id in idle_ids {
            sessions.remove(&id);
        }
        count
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
}

impl Default for SSHSessionManager {
    fn default() -> Self {
        Self::new(100, 3600) // 100 sessions, 1 hour idle timeout
    }
}

// =============================================================================
// Errors
// =============================================================================

/// Session management errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Maximum number of sessions reached")]
    MaxSessionsReached,

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Session is not active")]
    SessionNotActive,

    #[error("Command execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Command timed out after {0}ms")]
    Timeout(u64),
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = SSHSession::new("oxidized".to_string(), "/app".to_string());
        
        assert_eq!(session.user, "oxidized");
        assert_eq!(session.cwd, "/app");
        assert_eq!(session.state, SessionState::Active);
        assert!(session.pid.is_none());
    }

    #[test]
    fn test_session_idle_detection() {
        let mut session = SSHSession::new("user".to_string(), "/".to_string());
        
        // Just created, should not be idle
        assert!(!session.is_idle(1));
        
        // Manually set last_activity to the past
        session.last_activity = Utc::now() - chrono::Duration::seconds(10);
        
        // Now should be idle for 5 second threshold
        assert!(session.is_idle(5));
        // But not for 60 second threshold
        assert!(!session.is_idle(60));
    }

    #[test]
    fn test_command_request_builder() {
        let request = CommandRequest::new("ls -la")
            .with_cwd("/app")
            .with_timeout(5000)
            .with_env("PATH", "/usr/bin");
        
        assert_eq!(request.command, "ls -la");
        assert_eq!(request.cwd, Some("/app".to_string()));
        assert_eq!(request.timeout_ms, 5000);
        assert_eq!(request.env.get("PATH"), Some(&"/usr/bin".to_string()));
    }

    #[test]
    fn test_command_result_success() {
        let result = CommandResult {
            session_id: Uuid::new_v4(),
            command: "echo hello".to_string(),
            stdout: "hello\n".to_string(),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: 10,
            timed_out: false,
            completed_at: Utc::now(),
        };
        
        assert!(result.success());
        assert_eq!(result.combined_output(), "hello\n");
    }

    #[tokio::test]
    async fn test_session_manager() {
        let manager = SSHSessionManager::new(10, 3600);
        
        // Register a session
        let session = SSHSession::new("test".to_string(), "/".to_string());
        let id = manager.register_session(session).await.unwrap();
        
        // Should be able to get it
        let retrieved = manager.get_session(&id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().user, "test");
        
        // Update activity
        assert!(manager.update_activity(&id).await);
        
        // Remove it
        let removed = manager.remove_session(&id).await;
        assert!(removed.is_some());
        
        // Should not exist anymore
        assert!(manager.get_session(&id).await.is_none());
    }

    #[tokio::test]
    async fn test_session_manager_max_sessions() {
        let manager = SSHSessionManager::new(2, 3600);
        
        // Register 2 sessions
        manager.register_session(SSHSession::new("u1".to_string(), "/".to_string())).await.unwrap();
        manager.register_session(SSHSession::new("u2".to_string(), "/".to_string())).await.unwrap();
        
        // Third should fail
        let result = manager.register_session(SSHSession::new("u3".to_string(), "/".to_string())).await;
        assert!(matches!(result, Err(SessionError::MaxSessionsReached)));
    }
}
