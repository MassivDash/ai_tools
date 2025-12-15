use crate::api::agent::types::{ChatMessage, MessageRole};
use anyhow::{Context, Result};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::path::Path;

/// SQLite-based conversation storage
/// Only stores user and assistant messages (not tool calls or internal thoughts)
pub struct SqliteConversationMemory {
    pool: SqlitePool,
}

impl SqliteConversationMemory {
    /// Create a new SQLite conversation memory store
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();

        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        // Get absolute path - if file doesn't exist yet, canonicalize parent and join filename
        let absolute_path = if db_path.exists() {
            db_path
                .canonicalize()
                .context("Failed to canonicalize existing database path")?
        } else {
            let parent = db_path.parent().unwrap_or(Path::new("."));
            let parent_abs = parent
                .canonicalize()
                .or_else(|_| std::env::current_dir().map(|d| d.join(parent)))
                .context("Failed to get absolute path for database directory")?;
            let filename = db_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("conversations.db");
            parent_abs.join(filename)
        };

        println!(
            "💾 Connecting to SQLite database at: {}",
            absolute_path.display()
        );

        // Use SqliteConnectOptions with filename directly
        let options = SqliteConnectOptions::new()
            .filename(&absolute_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options).await.context(format!(
            "Failed to connect to SQLite database at: {}",
            absolute_path.display()
        ))?;

        // Create tables
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
        )
        .execute(&pool)
        .await
        .context("Failed to create conversations table")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
        )
        .execute(&pool)
        .await
        .context("Failed to create messages table")?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id)",
        )
        .execute(&pool)
        .await
        .context("Failed to create index")?;

        Ok(Self { pool })
    }

    /// Get or create a conversation ID
    pub async fn get_or_create_conversation_id(
        &self,
        conversation_id: Option<String>,
    ) -> Result<String> {
        if let Some(id) = conversation_id {
            // Check if conversation exists
            let exists: Option<i64> =
                sqlx::query_scalar("SELECT 1 FROM conversations WHERE id = ?1")
                    .bind(&id)
                    .fetch_optional(&self.pool)
                    .await
                    .context("Failed to check conversation existence")?;

            if exists.is_none() {
                // Create new conversation
                sqlx::query("INSERT INTO conversations (id) VALUES (?1)")
                    .bind(&id)
                    .execute(&self.pool)
                    .await
                    .context("Failed to create conversation")?;
            }

            Ok(id)
        } else {
            // Generate a new conversation ID
            use uuid::Uuid;
            let id = Uuid::new_v4().to_string();

            sqlx::query("INSERT INTO conversations (id) VALUES (?1)")
                .bind(&id)
                .execute(&self.pool)
                .await
                .context("Failed to create conversation")?;

            Ok(id)
        }
    }

    /// Add a user or assistant message (filters out tool calls and system messages)
    pub async fn add_message(&self, conversation_id: &str, message: ChatMessage) -> Result<()> {
        // Only store user and assistant messages
        match message.role {
            MessageRole::User | MessageRole::Assistant => {
                let role_str = match message.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    _ => return Ok(()), // Skip other roles
                };

                sqlx::query(
                    "INSERT INTO messages (conversation_id, role, content) VALUES (?1, ?2, ?3)",
                )
                .bind(conversation_id)
                .bind(role_str)
                .bind(&message.content)
                .execute(&self.pool)
                .await
                .context("Failed to insert message")?;
            }
            _ => {
                // Skip system, tool, and other message types
            }
        }

        Ok(())
    }

    /// Get all user and assistant messages for a conversation
    pub async fn get_messages(&self, conversation_id: &str) -> Result<Vec<ChatMessage>> {
        let rows = sqlx::query(
            "SELECT role, content FROM messages 
             WHERE conversation_id = ?1 
             ORDER BY created_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch messages")?;

        let mut messages = Vec::new();
        for row in rows {
            let role_str: String = row.get(0);
            let content: String = row.get(1);

            let role = match role_str.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::User, // Default fallback
            };

            messages.push(ChatMessage {
                role,
                content,
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }

        Ok(messages)
    }

    /// Clear a conversation
    /// Useful for admin/debugging purposes
    pub async fn clear_conversation(&self, conversation_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM conversations WHERE id = ?1")
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .context("Failed to clear conversation")?;

        Ok(())
    }

    /// Get the number of messages in a conversation
    /// Useful for debugging and monitoring
    pub async fn message_count(&self, conversation_id: &str) -> Result<usize> {
        let count: Option<i64> =
            sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE conversation_id = ?1")
                .bind(conversation_id)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to count messages")?;

        Ok(count.unwrap_or(0) as usize)
    }
}
