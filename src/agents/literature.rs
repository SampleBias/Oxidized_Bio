//! Literature Agent
//! 
//! Searches scientific literature and databases for relevant information.
//! In a full implementation, this would integrate with:
//! - PubMed/PubMed Central
//! - OpenScholar
//! - UniProt, PubChem
//! - ClinicalTrials.gov
//! - Patent databases
//!
//! For now, this uses LLM knowledge as a fallback.

use crate::models::PlanTask;
use crate::types::{LLMRequest, LLMMessage, AppResult};
use crate::llm::provider::{LLMProviderConfig, LLM};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

/// Result from a literature search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiteratureResult {
    pub task_id: String,
    pub objective: String,
    pub findings: String,
    pub sources: Vec<SourceReference>,
    pub key_insights: Vec<String>,
}

/// Reference to a source/paper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReference {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authors: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub summary: String,
}

/// Raw JSON response from LLM for literature search
#[derive(Debug, Deserialize)]
struct LiteratureLLMResponse {
    findings: String,
    #[serde(default)]
    sources: Vec<SourceRaw>,
    key_insights: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SourceRaw {
    title: String,
    authors: Option<String>,
    year: Option<i32>,
    doi: Option<String>,
    url: Option<String>,
    summary: String,
}

pub struct LiteratureAgent;

impl LiteratureAgent {
    /// Execute a literature search task
    pub async fn execute_task(
        task: &PlanTask,
        config: &crate::config::Config,
    ) -> AppResult<LiteratureResult> {
        let task_id = task.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        info!(task_id = %task_id, objective = %task.objective, "Starting literature search");

        // Get LLM provider configuration
        let api_key = config.llm.openai_api_key.clone();
        if api_key.is_empty() {
            warn!("No OpenAI API key configured, using placeholder response");
            return Ok(Self::placeholder_result(task));
        }

        // Create the search prompt
        let prompt = Self::create_search_prompt(&task.objective);

        // Create LLM request
        let llm = LLM::new(LLMProviderConfig {
            name: config.llm.default_provider.clone(),
            api_key,
        });

        let request = LLMRequest {
            provider: config.llm.default_provider.clone(),
            model: config.llm.default_model.clone(),
            messages: vec![LLMMessage::user(&prompt)],
            max_tokens: Some(2048),
            temperature: Some(0.3), // Lower temperature for more factual responses
            system_instruction: Some(
                "You are a scientific literature research assistant with deep knowledge of biology, medicine, and life sciences. Provide accurate, evidence-based information with citations where possible.".to_string()
            ),
        };

        match llm.create_chat_completion(&request).await {
            Ok(response) => {
                info!(response_len = response.content.len(), "Received literature response from LLM");
                
                // Parse the JSON response
                match Self::parse_literature_response(&response.content, task) {
                    Ok(result) => {
                        info!(
                            task_id = %result.task_id,
                            source_count = result.sources.len(),
                            insight_count = result.key_insights.len(),
                            "Literature search completed successfully"
                        );
                        Ok(result)
                    }
                    Err(e) => {
                        warn!(error = %e, "Failed to parse literature response, extracting text");
                        // Fall back to treating the entire response as findings
                        Ok(LiteratureResult {
                            task_id,
                            objective: task.objective.clone(),
                            findings: response.content,
                            sources: vec![],
                            key_insights: vec![],
                        })
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "LLM call failed, using placeholder response");
                Ok(Self::placeholder_result(task))
            }
        }
    }

    /// Placeholder result when LLM is not available
    fn placeholder_result(task: &PlanTask) -> LiteratureResult {
        LiteratureResult {
            task_id: task.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            objective: task.objective.clone(),
            findings: format!(
                "Literature search pending for: {}. Please configure an LLM API key to enable full search functionality.",
                task.objective
            ),
            sources: vec![],
            key_insights: vec![
                "LLM API key required for full literature search".to_string(),
            ],
        }
    }

    /// Create the literature search prompt
    fn create_search_prompt(objective: &str) -> String {
        format!(r#"You are conducting a scientific literature search. Based on your knowledge, provide comprehensive information about the following research question:

RESEARCH OBJECTIVE:
{objective}

Provide a thorough, evidence-based response covering:
1. Key findings from relevant research
2. Important studies and their conclusions
3. Current scientific consensus
4. Any controversies or gaps in knowledge

OUTPUT FORMAT (respond with ONLY valid JSON):
{{
  "findings": "Detailed narrative of key findings (2-4 paragraphs with inline citations where possible)",
  "sources": [
    {{
      "title": "Study/Paper title",
      "authors": "Author names if known",
      "year": 2023,
      "doi": "DOI if known or null",
      "url": "URL if known or null",
      "summary": "Brief summary of this source's relevance"
    }}
  ],
  "key_insights": [
    "Key insight 1",
    "Key insight 2",
    "Key insight 3"
  ]
}}

IMPORTANT:
- Be factual and evidence-based
- Include specific mechanisms, dosages, or effects where known
- Cite studies when possible (title, authors, year)
- Note limitations or areas of uncertainty
- Respond with ONLY the JSON object"#,
            objective = objective
        )
    }

    /// Parse the LLM response into a LiteratureResult
    fn parse_literature_response(response: &str, task: &PlanTask) -> Result<LiteratureResult> {
        // Try to extract JSON from the response
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

        let parsed: LiteratureLLMResponse = serde_json::from_str(json_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse literature JSON: {}", e))?;

        let sources = parsed
            .sources
            .into_iter()
            .map(|s| SourceReference {
                title: s.title,
                authors: s.authors,
                year: s.year,
                doi: s.doi,
                url: s.url,
                summary: s.summary,
            })
            .collect();

        Ok(LiteratureResult {
            task_id: task.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            objective: task.objective.clone(),
            findings: parsed.findings,
            sources,
            key_insights: parsed.key_insights,
        })
    }

    /// Format literature results for inclusion in reply context
    pub fn format_for_reply(results: &[LiteratureResult]) -> String {
        if results.is_empty() {
            return "No literature search results available.".to_string();
        }

        let mut output = String::new();
        
        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!("\n### Literature Search {}\n", i + 1));
            output.push_str(&format!("**Objective:** {}\n\n", result.objective));
            output.push_str(&format!("**Findings:**\n{}\n\n", result.findings));
            
            if !result.key_insights.is_empty() {
                output.push_str("**Key Insights:**\n");
                for insight in &result.key_insights {
                    output.push_str(&format!("- {}\n", insight));
                }
                output.push_str("\n");
            }
            
            if !result.sources.is_empty() {
                output.push_str("**Sources:**\n");
                for source in &result.sources {
                    let citation = match (&source.doi, &source.url) {
                        (Some(doi), _) => format!(" [DOI: {}]", doi),
                        (_, Some(url)) => format!(" [{}]", url),
                        _ => String::new(),
                    };
                    let year = source.year.map(|y| format!(" ({})", y)).unwrap_or_default();
                    output.push_str(&format!("- {}{}{}\n", source.title, year, citation));
                }
            }
        }
        
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_for_reply_empty() {
        let result = LiteratureAgent::format_for_reply(&[]);
        assert!(result.contains("No literature search results"));
    }

    #[test]
    fn test_format_for_reply_with_results() {
        let results = vec![LiteratureResult {
            task_id: "test-1".to_string(),
            objective: "Test objective".to_string(),
            findings: "Test findings".to_string(),
            sources: vec![SourceReference {
                title: "Test Study".to_string(),
                authors: Some("Author A".to_string()),
                year: Some(2023),
                doi: Some("10.1234/test".to_string()),
                url: None,
                summary: "Test summary".to_string(),
            }],
            key_insights: vec!["Insight 1".to_string()],
        }];

        let formatted = LiteratureAgent::format_for_reply(&results);
        assert!(formatted.contains("Test objective"));
        assert!(formatted.contains("Test findings"));
        assert!(formatted.contains("Test Study"));
        assert!(formatted.contains("10.1234/test"));
    }
}
