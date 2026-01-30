// Planning agent stub
// TODO: Implement full planning agent for research plan generation

use crate::models::{ConversationState, Message, PlanTask, ConversationStateValues};
use anyhow::Result;

pub struct PlanningAgent;

impl PlanningAgent {
    pub async fn generate_plan(
        message: &Message,
        conversation_state: &ConversationState,
    ) -> Result<Vec<PlanTask>> {
        // Placeholder implementation
        Ok(vec![])
    }

    pub async fn generate_next_plan(
        message: &Message,
        conversation_state: &ConversationState,
    ) -> Result<Vec<PlanTask>> {
        // Placeholder implementation
        Ok(vec![])
    }
}
