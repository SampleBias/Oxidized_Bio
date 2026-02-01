//! Planning Agent
//! 
//! Analyzes user queries and generates a research plan with tasks.
//! This is the first step in the agent pipeline.

use crate::models::{ConversationState, Message, PlanTask, DatasetRef};
use crate::types::{LLMRequest, LLMMessage, AppResult, AppError};
use crate::llm::provider::{LLMProviderConfig, LLM};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

/// Planning agent result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningResult {
    pub current_objective: String,
    pub plan: Vec<PlanTask>,
}

/// Raw JSON response from LLM
#[derive(Debug, Deserialize)]
struct PlanningLLMResponse {
    #[serde(rename = "currentObjective")]
    current_objective: String,
    plan: Vec<PlanTaskRaw>,
}

#[derive(Debug, Deserialize)]
struct PlanTaskRaw {
    objective: String,
    #[serde(default)]
    datasets: Vec<DatasetRefRaw>,
    #[serde(rename = "type")]
    task_type: String,
}

#[derive(Debug, Deserialize)]
struct DatasetRefRaw {
    filename: String,
    id: String,
    description: String,
}

pub struct PlanningAgent;

impl PlanningAgent {
    /// Generate a research plan based on user's message
    pub async fn generate_plan(
        message: &str,
        conversation_state: Option<&ConversationState>,
        config: &crate::config::Config,
    ) -> AppResult<PlanningResult> {
        info!(message_len = message.len(), "Starting planning agent");

        // Get LLM provider configuration
        let api_key = match config.llm.active_api_key() {
            Some(key) => key,
            None => {
                warn!("No LLM API key configured, using simple planning fallback");
                return Ok(Self::simple_plan(message));
            }
        };

        // Build context from conversation state
        let context = Self::build_context(conversation_state);

        // Create the planning prompt
        let prompt = Self::create_planning_prompt(message, &context);

        // Create LLM request
        let llm = LLM::new(LLMProviderConfig {
            name: config.llm.default_provider.clone(),
            api_key,
        });

        let request = LLMRequest {
            provider: config.llm.default_provider.clone(),
            model: config.llm.default_model.clone(),
            messages: vec![LLMMessage::user(&prompt)],
            max_tokens: Some(1024),
            temperature: Some(0.7),
            system_instruction: None,
        };

        match llm.create_chat_completion(&request).await {
            Ok(response) => {
                info!(response_len = response.content.len(), "Received planning response from LLM");
                
                // Parse the JSON response
                match Self::parse_planning_response(&response.content) {
                    Ok(result) => {
                        info!(
                            objective = %result.current_objective,
                            task_count = result.plan.len(),
                            "Planning completed successfully"
                        );
                        Ok(result)
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to parse planning response, using fallback");
                        Ok(Self::simple_plan(message))
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "LLM call failed, using fallback planning");
                Ok(Self::simple_plan(message))
            }
        }
    }

    /// Simple fallback plan when LLM is not available
    fn simple_plan(message: &str) -> PlanningResult {
        PlanningResult {
            current_objective: format!("Research: {}", message),
            plan: vec![PlanTask {
                id: Some(uuid::Uuid::new_v4().to_string()),
                job_id: None,
                objective: message.to_string(),
                datasets: vec![],
                task_type: "LITERATURE".to_string(),
                level: Some(1),
                start: None,
                end: None,
                output: None,
                artifacts: None,
            }],
        }
    }

    /// Build context string from conversation state
    fn build_context(conversation_state: Option<&ConversationState>) -> String {
        match conversation_state {
            Some(state) => {
                let mut context = String::new();
                
                if let Some(obj) = &state.values.current_objective {
                    context.push_str(&format!("Current Objective: {}\n", obj));
                }
                
                if let Some(insights) = &state.values.key_insights {
                    if !insights.is_empty() {
                        context.push_str("Key Insights:\n");
                        for insight in insights {
                            context.push_str(&format!("- {}\n", insight));
                        }
                    }
                }
                
                if let Some(hypothesis) = &state.values.current_hypothesis {
                    context.push_str(&format!("\nCurrent Hypothesis: {}\n", hypothesis));
                }
                
                if let Some(datasets) = &state.values.uploaded_datasets {
                    if !datasets.is_empty() {
                        context.push_str("\nUploaded Datasets:\n");
                        for ds in datasets {
                            context.push_str(&format!("- {} ({}): {}\n", ds.filename, ds.id, ds.description));
                        }
                    }
                }
                
                if context.is_empty() {
                    "No existing research context.".to_string()
                } else {
                    context
                }
            }
            None => "No existing research context.".to_string(),
        }
    }

    /// Create the planning prompt for the LLM
    fn create_planning_prompt(message: &str, context: &str) -> String {
        format!(r#"You are a research planning agent. Analyze the user's question and create a simple research plan.

CURRENT RESEARCH STATE:
{context}

USER'S MESSAGE:
{message}

AVAILABLE TASK TYPES:
- LITERATURE: Search and gather scientific papers and knowledge. Use for finding research, papers, clinical data, mechanisms.

Create a focused plan with 1-2 tasks maximum.

OUTPUT FORMAT (respond with ONLY valid JSON):
{{
  "currentObjective": "Brief description of what we're researching (1 sentence)",
  "plan": [
    {{
      "objective": "Specific search objective",
      "datasets": [],
      "type": "LITERATURE"
    }}
  ]
}}

Respond with ONLY the JSON object, no additional text."#,
            context = context,
            message = message
        )
    }

    /// Parse the LLM response into a PlanningResult
    fn parse_planning_response(response: &str) -> Result<PlanningResult> {
        // Try to extract JSON from the response (handle markdown code blocks)
        let json_str = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
                .trim()
        } else if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .unwrap_or(response)
                .trim()
        } else {
            response.trim()
        };

        let parsed: PlanningLLMResponse = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse planning JSON: {}", e))?;

        let plan = parsed
            .plan
            .into_iter()
            .map(|task| PlanTask {
                id: Some(uuid::Uuid::new_v4().to_string()),
                job_id: None,
                objective: task.objective,
                datasets: task
                    .datasets
                    .into_iter()
                    .map(|d| DatasetRef {
                        filename: d.filename,
                        id: d.id,
                        description: d.description,
                        path: None,
                    })
                    .collect(),
                task_type: task.task_type,
                level: Some(1),
                start: None,
                end: None,
                output: None,
                artifacts: None,
            })
            .collect();

        Ok(PlanningResult {
            current_objective: parsed.current_objective,
            plan,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_planning_response() {
        let response = r#"{"currentObjective":"Research rapamycin effects","plan":[{"objective":"Find studies on rapamycin and longevity","datasets":[],"type":"LITERATURE"}]}"#;
        
        let result = PlanningAgent::parse_planning_response(response).unwrap();
        assert_eq!(result.current_objective, "Research rapamycin effects");
        assert_eq!(result.plan.len(), 1);
        assert_eq!(result.plan[0].task_type, "LITERATURE");
    }

    #[test]
    fn test_simple_plan() {
        let result = PlanningAgent::simple_plan("What are the effects of metformin?");
        assert!(!result.current_objective.is_empty());
        assert_eq!(result.plan.len(), 1);
    }
}
