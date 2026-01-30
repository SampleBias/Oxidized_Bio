// Reply agent stub
// TODO: Implement full reply agent for user-facing responses

use crate::models::{ConversationState, Message};
use anyhow::Result;

pub struct ReplyAgent;

impl ReplyAgent {
    pub async fn generate_reply(
        message: &Message,
        conversation_state: &ConversationState,
        is_deep_research: bool,
    ) -> Result<String> {
        // Placeholder implementation
        Ok("I'm working on your request.".to_string())
    }
}
