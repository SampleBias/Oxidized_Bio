use sqlx::PgPool;
use crate::models::*;
use anyhow::Result;
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub struct DatabaseOperations;

impl DatabaseOperations {
    // User operations
    pub async fn get_or_create_user(
        pool: &PgPool,
        wallet_address: Option<String>,
    ) -> Result<User> {
        if let Some(wallet) = wallet_address {
            // Try to find by wallet address
            if let Some(user) = Self::get_user_by_wallet(pool, &wallet).await? {
                return Ok(user);
            }
        }

        // Create new user
        let user_id = Uuid::new_v4();
        let email = format!("{}@oxidized.bio", user_id);
        let username = format!("user_{}", user_id);

        let row = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, username, email, wallet_address)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
            user_id,
            username,
            email,
            wallet_address
        )
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    pub async fn get_user_by_wallet(pool: &PgPool, wallet: &str) -> Result<Option<User>> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM users WHERE wallet_address = $1",
            wallet
        )
        .fetch_optional(pool)
        .await?;

        Ok(user)
    }

    // Conversation operations
    pub async fn create_conversation(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Conversation> {
        // Create conversation state first
        let state_id = sqlx::query_scalar!(
            "INSERT INTO conversation_states (values) VALUES ($1) RETURNING id",
            serde_json::json!({})
        )
        .fetch_one(pool)
        .await?;

        // Create conversation
        let conversation = sqlx::query_as!(
            Conversation,
            r#"
            INSERT INTO conversations (user_id, conversation_state_id)
            VALUES ($1, $2)
            RETURNING *
            "#,
            user_id,
            state_id
        )
        .fetch_one(pool)
        .await?;

        Ok(conversation)
    }

    pub async fn get_conversation(
        pool: &PgPool,
        conversation_id: Uuid,
    ) -> Result<Option<Conversation>> {
        let conv = sqlx::query_as!(
            Conversation,
            "SELECT * FROM conversations WHERE id = $1",
            conversation_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(conv)
    }

    // Message operations
    pub async fn create_message(
        pool: &PgPool,
        message: &Message,
    ) -> Result<Message> {
        let new_message = sqlx::query_as!(
            Message,
            r#"
            INSERT INTO messages (conversation_id, user_id, question, content, response_time, source, files)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
            message.conversation_id,
            message.user_id,
            message.question,
            message.content,
            message.response_time,
            message.source,
            message.files
        )
        .fetch_one(pool)
        .await?;

        Ok(new_message)
    }

    pub async fn get_messages_for_conversation(
        pool: &PgPool,
        conversation_id: Uuid,
    ) -> Result<Vec<Message>> {
        let messages = sqlx::query_as!(
            Message,
            r#"
            SELECT * FROM messages
            WHERE conversation_id = $1
            ORDER BY created_at ASC
            "#,
            conversation_id
        )
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    // Conversation state operations
    pub async fn update_conversation_state(
        pool: &PgPool,
        state_id: Uuid,
        values: &ConversationStateValues,
    ) -> Result<()> {
        let json_value = serde_json::to_value(values)?;

        sqlx::query!(
            r#"
            UPDATE conversation_states
            SET values = $1, updated_at = NOW()
            WHERE id = $2
            "#,
            json_value,
            state_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_conversation_state(
        pool: &PgPool,
        state_id: Uuid,
    ) -> Result<Option<ConversationState>> {
        let state = sqlx::query!(
            "SELECT id, values FROM conversation_states WHERE id = $1",
            state_id
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = state {
            let values: ConversationStateValues = serde_json::from_value(row.values)?;
            Ok(Some(ConversationState {
                id: Some(row.id),
                values,
            }))
        } else {
            Ok(None)
        }
    }

    // Token usage tracking
    pub async fn create_token_usage(
        pool: &PgPool,
        message_id: Option<Uuid>,
        provider: &str,
        model: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
        total_tokens: i32,
        duration_ms: i32,
    ) -> Result<()> {
        // This would require a token_usages table - for now it's a placeholder
        // In the TypeScript version, this creates a token_usage record
        tracing::info!(
            message_id = ?message_id,
            provider,
            model,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            duration_ms,
            "Token usage logged"
        );
        Ok(())
    }
}
