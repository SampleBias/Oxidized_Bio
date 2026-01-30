use sqlx::PgPool;
use crate::config::Config;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}

// Core models based on TypeScript definitions

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: uuid::Uuid,
    pub username: String,
    pub email: String,
    pub wallet_address: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub id: Option<uuid::Uuid>,
    pub conversation_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub question: Option<String>,
    pub content: String,
    pub response_time: Option<i32>,
    pub source: Option<String>,
    pub files: Option<serde_json::Value>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Conversation {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub conversation_state_id: Option<uuid::Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationState {
    pub id: Option<uuid::Uuid>,
    pub values: ConversationStateValues,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationStateValues {
    pub objective: String,
    pub conversation_title: Option<String>,
    pub current_objective: Option<String>,
    pub current_level: Option<i32>,
    pub key_insights: Option<Vec<String>>,
    pub methodology: Option<String>,
    pub current_hypothesis: Option<String>,
    pub discoveries: Option<Vec<Discovery>>,
    pub plan: Option<Vec<PlanTask>>,
    pub suggested_next_steps: Option<Vec<PlanTask>>,
    pub research_mode: Option<String>,
    pub uploaded_datasets: Option<Vec<UploadedDataset>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateValues {
    pub message_id: Option<uuid::Uuid>,
    pub conversation_id: Option<uuid::Uuid>,
    pub user_id: Option<uuid::Uuid>,
    pub source: Option<String>,
    pub is_deep_research: Option<bool>,
    pub final_response: Option<String>,
    pub thought: Option<String>,
    pub steps: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanTask {
    pub id: Option<String>,
    pub job_id: Option<String>,
    pub objective: String,
    pub datasets: Vec<DatasetRef>,
    pub task_type: String, // "LITERATURE" or "ANALYSIS"
    pub level: Option<i32>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub output: Option<String>,
    pub artifacts: Option<Vec<AnalysisArtifact>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DatasetRef {
    pub filename: String,
    pub id: String,
    pub description: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UploadedDataset {
    pub filename: String,
    pub id: String,
    pub description: String,
    pub path: Option<String>,
    pub content: Option<String>,
    pub size: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Discovery {
    pub title: String,
    pub claim: String,
    pub summary: String,
    pub evidence_array: Vec<DiscoveryEvidence>,
    pub artifacts: Vec<AnalysisArtifact>,
    pub novelty: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryEvidence {
    pub task_id: String,
    pub job_id: Option<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisArtifact {
    pub id: String,
    pub description: String,
    pub artifact_type: String, // "FILE" or "FOLDER"
    pub content: Option<String>,
    pub name: String,
    pub path: Option<String>,
}

// API Request/Response types

#[derive(Debug, serde::Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub conversation_id: Option<uuid::Uuid>,
    pub files: Option<Vec<FileUpload>>,
}

#[derive(Debug, serde::Deserialize)]
pub struct FileUpload {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

#[derive(Debug, serde::Serialize)]
pub struct ChatResponse {
    pub message_id: uuid::Uuid,
    pub content: String,
    pub conversation_id: uuid::Uuid,
    pub response_time: i32,
}

#[derive(Debug, serde::Deserialize)]
pub struct DeepResearchRequest {
    pub message: String,
    pub conversation_id: Option<uuid::Uuid>,
    pub research_mode: Option<String>, // "semi-autonomous", "fully-autonomous", "steering"
}

#[derive(Debug, serde::Serialize)]
pub struct DeepResearchResponse {
    pub message_id: uuid::Uuid,
    pub status: String,
    pub conversation_id: uuid::Uuid,
}

#[derive(Debug, serde::Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub database: String,
    pub redis: Option<String>,
}
