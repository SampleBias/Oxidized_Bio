//! Reply Agent
//! 
//! Synthesizes research findings and generates user-facing responses.
//! This is the final step in the agent pipeline.

use crate::models::PlanTask;
use crate::types::{LLMRequest, LLMMessage, AppResult};
use crate::llm::provider::{LLMProviderConfig, LLM};
use crate::agents::literature::LiteratureResult;
use crate::agents::planning::PlanningResult;
use anyhow::Result;
use futures::StreamExt;
use tracing::{info, warn, error};

/// Reply mode - determines output format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplyMode {
    /// Direct answer to a question
    Answer,
    /// Research report with next steps
    Report,
    /// Simple chat response (non-research)
    Chat,
}

pub struct ReplyAgent;

impl ReplyAgent {
    fn max_tokens_for_provider(config: &crate::config::Config) -> u32 {
        if config.llm.default_provider == "groq" {
            4096
        } else {
            2048
        }
    }

    /// Generate a response based on research findings
    pub async fn generate_response(
        user_message: &str,
        planning: Option<&PlanningResult>,
        literature_results: &[LiteratureResult],
        mode: ReplyMode,
        config: &crate::config::Config,
    ) -> AppResult<String> {
        info!(
            message_len = user_message.len(),
            mode = ?mode,
            literature_count = literature_results.len(),
            "Generating reply"
        );

        // Get LLM provider configuration
        let api_key = match config.llm.active_api_key() {
            Some(key) => key,
            None => {
                warn!("No LLM API key configured, using simple response");
                return Ok(Self::simple_response(user_message, literature_results));
            }
        };

        // Build the prompt based on mode
        let prompt = match mode {
            ReplyMode::Answer => Self::create_answer_prompt(user_message, literature_results, planning),
            ReplyMode::Report => Self::create_report_prompt(user_message, literature_results, planning),
            ReplyMode::Chat => Self::create_chat_prompt(user_message, literature_results),
        };

        // Create LLM request
        let llm = LLM::new(LLMProviderConfig {
            name: config.llm.default_provider.clone(),
            api_key,
        });

        let request = LLMRequest {
            provider: config.llm.default_provider.clone(),
            model: config.llm.default_model.clone(),
            messages: vec![LLMMessage::user(&prompt)],
            max_tokens: Some(Self::max_tokens_for_provider(config)),
            temperature: Some(0.7),
            system_instruction: Some(
                "You are a knowledgeable research assistant. Provide clear, accurate, and helpful responses based on the research context provided.".to_string()
            ),
        };

        match llm.create_chat_completion(&request).await {
            Ok(response) => {
                info!(response_len = response.content.len(), "Generated reply successfully");
                Ok(response.content)
            }
            Err(e) => {
                error!(error = %e, "LLM call failed, using simple response");
                Ok(Self::simple_response(user_message, literature_results))
            }
        }
    }

    /// Generate a response with streaming support (chunks sent via callback)
    pub async fn generate_response_streaming<F>(
        user_message: &str,
        planning: Option<&PlanningResult>,
        literature_results: &[LiteratureResult],
        mode: ReplyMode,
        config: &crate::config::Config,
        mut on_chunk: F,
    ) -> AppResult<String>
    where
        F: FnMut(&str) + Send,
    {
        info!(
            message_len = user_message.len(),
            mode = ?mode,
            literature_count = literature_results.len(),
            "Generating reply (streaming)"
        );

        let api_key = match config.llm.active_api_key() {
            Some(key) => key,
            None => {
                warn!("No LLM API key configured, using simple response");
                return Ok(Self::simple_response(user_message, literature_results));
            }
        };

        let prompt = match mode {
            ReplyMode::Answer => Self::create_answer_prompt(user_message, literature_results, planning),
            ReplyMode::Report => Self::create_report_prompt(user_message, literature_results, planning),
            ReplyMode::Chat => Self::create_chat_prompt(user_message, literature_results),
        };

        let llm = LLM::new(LLMProviderConfig {
            name: config.llm.default_provider.clone(),
            api_key,
        });

        let request = LLMRequest {
            provider: config.llm.default_provider.clone(),
            model: config.llm.default_model.clone(),
            messages: vec![LLMMessage::user(&prompt)],
            max_tokens: Some(Self::max_tokens_for_provider(config)),
            temperature: Some(0.7),
            system_instruction: Some(
                "You are a knowledgeable research assistant. Provide clear, accurate, and helpful responses based on the research context provided.".to_string()
            ),
        };

        match llm.create_chat_completion_stream(&request).await {
            Ok(mut stream) => {
                let mut full = String::new();
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(delta) => {
                            if !delta.is_empty() {
                                on_chunk(&delta);
                                full.push_str(&delta);
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Streaming chunk failed, falling back");
                            return Self::generate_response(user_message, planning, literature_results, mode, config).await;
                        }
                    }
                }

                if full.is_empty() {
                    warn!("Streaming returned empty response, falling back");
                    return Self::generate_response(user_message, planning, literature_results, mode, config).await;
                }

                Ok(full)
            }
            Err(e) => {
                warn!(error = %e, "Streaming not available, falling back to standard completion");
                Self::generate_response(user_message, planning, literature_results, mode, config).await
            }
        }
    }

    /// Simple fallback response when LLM is not available
    fn simple_response(user_message: &str, literature_results: &[LiteratureResult]) -> String {
        if literature_results.is_empty() {
            format!(
                "I received your question: \"{}\"\n\n\
                To provide you with accurate, evidence-based information, please ensure an LLM API key is configured.\n\n\
                In the meantime, I can help you refine your question or suggest related topics to explore.",
                user_message
            )
        } else {
            let mut response = format!(
                "## Research Results\n\n\
                Based on your question: \"{}\"\n\n",
                user_message
            );

            for result in literature_results {
                response.push_str(&format!("### {}\n\n", result.objective));
                response.push_str(&format!("{}\n\n", result.findings));
                
                if !result.key_insights.is_empty() {
                    response.push_str("**Key Insights:**\n");
                    for insight in &result.key_insights {
                        response.push_str(&format!("- {}\n", insight));
                    }
                    response.push_str("\n");
                }
            }

            response.push_str("\n---\n\n*Let me know if you'd like me to explore any aspect further!*");
            response
        }
    }

    /// Create prompt for answer mode (direct questions)
    fn create_answer_prompt(
        question: &str,
        literature_results: &[LiteratureResult],
        planning: Option<&PlanningResult>,
    ) -> String {
        let literature_context = crate::agents::LiteratureAgent::format_for_reply(literature_results);
        
        let next_steps = planning
            .map(|p| {
                if p.plan.is_empty() {
                    "No further research planned.".to_string()
                } else {
                    p.plan
                        .iter()
                        .map(|t| format!("- {}", t.objective))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            })
            .unwrap_or_else(|| "No research plan available.".to_string());

        format!(r#"You are a research assistant answering a user's question using evidence gathered from scientific literature.

QUESTION: {question}

RESEARCH FINDINGS:
{literature_context}

PLANNED NEXT STEPS:
{next_steps}

TASK:
Generate a clear, helpful answer to the user's question based on the research findings.

GUIDELINES:
- Lead with a DIRECT ANSWER to their question
- Support claims with evidence from the research findings
- Include inline citations where available using format (claim)[DOI/URL]
- Be clear and accessible - avoid unnecessary jargon
- If the question cannot be fully answered, acknowledge limitations
- Suggest next steps if appropriate

OUTPUT FORMAT:

## [Title referencing the question]

[2-4 paragraphs directly answering the question with inline citations]

## Next Steps

[Brief description of planned research, or what additional research could help]

---

**Let me know if you'd like me to explore any aspect further!**"#,
            question = question,
            literature_context = literature_context,
            next_steps = next_steps,
        )
    }

    /// Create prompt for report mode (research directives)
    fn create_report_prompt(
        directive: &str,
        literature_results: &[LiteratureResult],
        planning: Option<&PlanningResult>,
    ) -> String {
        let literature_context = crate::agents::LiteratureAgent::format_for_reply(literature_results);
        
        let (current_objective, next_steps) = planning
            .map(|p| {
                let obj = p.current_objective.clone();
                let steps = if p.plan.is_empty() {
                    "Research complete - no additional tasks planned.".to_string()
                } else {
                    p.plan
                        .iter()
                        .map(|t| format!("- {} ({})", t.objective, t.task_type))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                (obj, steps)
            })
            .unwrap_or_else(|| ("Analyzing request".to_string(), "No plan available.".to_string()));

        format!(r#"You are a research assistant reporting results and next steps to the user.

USER'S REQUEST: {directive}

CURRENT OBJECTIVE: {current_objective}

RESEARCH FINDINGS:
{literature_context}

PLANNED NEXT STEPS:
{next_steps}

TASK:
Generate a user-facing research report that:
1. Summarizes what was done
2. Presents key findings and discoveries
3. Describes the current objective and next steps
4. Asks for user feedback

TONE:
- Conversational and friendly, but professional
- Clear and concise - avoid unnecessary jargon
- Use markdown formatting for structure
- Show enthusiasm for interesting findings
- Be transparent about limitations

OUTPUT FORMAT:

## What I Did

[Brief summary of the research performed]

## Key Findings

[Present the main discoveries and insights with citations]

## Current Objective & Next Steps

**Current Objective:** {current_objective}

Here's my plan for the next iteration:
[List planned tasks with brief reasoning]

## Summary

[One paragraph high-level overview]

---

**Let me know if you'd like me to proceed with this plan, or if you want to adjust the direction!**"#,
            directive = directive,
            current_objective = current_objective,
            literature_context = literature_context,
            next_steps = next_steps,
        )
    }

    /// Create prompt for chat mode (simple conversation)
    fn create_chat_prompt(
        message: &str,
        literature_results: &[LiteratureResult],
    ) -> String {
        let context = if literature_results.is_empty() {
            "No research context available.".to_string()
        } else {
            crate::agents::LiteratureAgent::format_for_reply(literature_results)
        };

        format!(r#"You are a knowledgeable research assistant having a conversation.

USER'S MESSAGE: {message}

AVAILABLE CONTEXT:
{context}

TASK:
Provide a helpful, conversational response to the user's message.

GUIDELINES:
- Be direct and concise
- Use evidence from the context if relevant
- Be helpful and friendly
- If asked something outside your knowledge, be honest about limitations
- Don't over-explain or be verbose

Respond naturally and helpfully."#,
            message = message,
            context = context,
        )
    }

    /// Classify reply mode based on the user message
    pub fn classify_mode(message: &str) -> ReplyMode {
        let message_lower = message.to_lowercase();
        
        // Check for explicit directives/commands
        let directive_indicators = [
            "research", "investigate", "analyze", "look into", "find papers",
            "study", "explore", "search for", "find studies", "review the literature",
        ];
        
        // Check for questions
        let question_indicators = [
            "what", "how", "why", "is there", "does", "can you explain",
            "tell me about", "what is", "what are", "could you",
        ];
        
        for indicator in directive_indicators {
            if message_lower.contains(indicator) {
                return ReplyMode::Report;
            }
        }
        
        for indicator in question_indicators {
            if message_lower.starts_with(indicator) || message_lower.contains(&format!("? {}", indicator)) {
                return ReplyMode::Answer;
            }
        }
        
        // Check if it ends with a question mark
        if message.trim().ends_with('?') {
            return ReplyMode::Answer;
        }
        
        // Default to Answer for most queries
        ReplyMode::Answer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_mode_question() {
        assert_eq!(ReplyAgent::classify_mode("What are the effects of metformin?"), ReplyMode::Answer);
        assert_eq!(ReplyAgent::classify_mode("How does rapamycin work?"), ReplyMode::Answer);
        assert_eq!(ReplyAgent::classify_mode("Is there evidence for NAD+ supplementation?"), ReplyMode::Answer);
    }

    #[test]
    fn test_classify_mode_directive() {
        assert_eq!(ReplyAgent::classify_mode("Research the effects of fasting on longevity"), ReplyMode::Report);
        assert_eq!(ReplyAgent::classify_mode("Investigate metformin mechanisms"), ReplyMode::Report);
        assert_eq!(ReplyAgent::classify_mode("Find papers on senolytics"), ReplyMode::Report);
    }

    #[test]
    fn test_simple_response() {
        let response = ReplyAgent::simple_response("test question", &[]);
        assert!(response.contains("test question"));
    }
}
