// Job definitions stub
// TODO: Define all job types for the queue

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub job_type: String,
    pub payload: serde_json::Value,
}

pub enum JobType {
    Chat,
    DeepResearch,
    FileUpload,
    LiteratureSearch,
    DataAnalysis,
}
