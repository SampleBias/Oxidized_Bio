//! Agent System
//! 
//! This module contains the core research agents that power the bio research assistant:
//! 
//! - **Planning Agent**: Analyzes user queries and creates research task plans
//! - **Literature Agent**: Searches scientific literature and databases
//! - **Reply Agent**: Synthesizes findings and generates user-facing responses
//! 
//! ## Pipeline Overview
//! 
//! ```text
//! User Message
//!      │
//!      ▼
//! ┌─────────────┐
//! │  Planning   │  → Generates research tasks
//! │   Agent     │
//! └─────────────┘
//!      │
//!      ▼
//! ┌─────────────┐
//! │ Literature  │  → Executes searches (in parallel)
//! │   Agent     │
//! └─────────────┘
//!      │
//!      ▼
//! ┌─────────────┐
//! │   Reply     │  → Synthesizes response
//! │   Agent     │
//! └─────────────┘
//!      │
//!      ▼
//!  User Response
//! ```

pub mod planning;
pub mod literature;
pub mod reply;
pub mod file_upload;
pub mod analysis;
pub mod hypothesis;
pub mod reflection;

// Re-export main components
pub use planning::{PlanningAgent, PlanningResult};
pub use literature::{LiteratureAgent, LiteratureResult, SourceReference};
pub use reply::{ReplyAgent, ReplyMode};
pub use file_upload::*;

use crate::models::PlanTask;
use crate::types::AppResult;
use tracing::info;

/// Execute the full research pipeline for a user message
pub async fn execute_research_pipeline(
    user_message: &str,
    conversation_state: Option<&crate::models::ConversationState>,
    config: &crate::config::Config,
) -> AppResult<String> {
    info!(message_len = user_message.len(), "Starting research pipeline");
    
    // Step 1: Planning - create research tasks
    let planning_result = PlanningAgent::generate_plan(
        user_message,
        conversation_state,
        config,
    ).await?;
    
    info!(
        objective = %planning_result.current_objective,
        task_count = planning_result.plan.len(),
        "Planning complete"
    );
    
    // Step 2: Execute literature tasks in parallel
    let literature_tasks: Vec<_> = planning_result
        .plan
        .iter()
        .filter(|t| t.task_type == "LITERATURE")
        .collect();
    
    let mut literature_results = Vec::new();
    for task in &literature_tasks {
        match LiteratureAgent::execute_task(task, config).await {
            Ok(result) => literature_results.push(result),
            Err(e) => {
                tracing::warn!(error = %e, task = ?task.objective, "Literature task failed");
            }
        }
    }
    
    info!(
        results_count = literature_results.len(),
        "Literature searches complete"
    );
    
    // Step 3: Generate reply
    let reply_mode = ReplyAgent::classify_mode(user_message);
    let response = ReplyAgent::generate_response(
        user_message,
        Some(&planning_result),
        &literature_results,
        reply_mode,
        config,
    ).await?;
    
    info!(response_len = response.len(), "Research pipeline complete");
    
    Ok(response)
}
