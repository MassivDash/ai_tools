use crate::api::agent::types::{ChatMessage, MessageRole, ToolCall};
use anyhow::{Context, Result};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::path::Path;

/// SQLite-based conversation storage
/// Stores all message types including tool calls and results
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

        // Check if messages table exists and has the new columns
        let table_exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='messages'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if table_exists.is_some() {
            // Check if tool_calls column exists
            let has_new_columns: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM pragma_table_info('messages') WHERE name='tool_calls'",
            )
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            if has_new_columns.is_none() {
                println!(
                    "⚠️  Detected outdated schema. Resetting messages table (Development Mode)..."
                );
                sqlx::query("DROP TABLE messages")
                    .execute(&pool)
                    .await
                    .context("Failed to drop old messages table")?;
            }
        }

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
                name TEXT,
                tool_calls TEXT,
                tool_call_id TEXT,
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

    /// Add a message to the conversation
    pub async fn add_message(&self, conversation_id: &str, message: ChatMessage) -> Result<()> {
        let role_str = match message.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        };

        let tool_calls_json = if let Some(calls) = &message.tool_calls {
            Some(serde_json::to_string(calls).context("Failed to serialize tool calls")?)
        } else {
            None
        };

        sqlx::query(
            "INSERT INTO messages (conversation_id, role, content, name, tool_calls, tool_call_id) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(conversation_id)
        .bind(role_str)
        .bind(&message.content)
        .bind(&message.name)
        .bind(&tool_calls_json)
        .bind(&message.tool_call_id)
        .execute(&self.pool)
        .await
        .context("Failed to insert message")?;

        Ok(())
    }

    /// Get all messages for a conversation
    pub async fn get_messages(&self, conversation_id: &str) -> Result<Vec<ChatMessage>> {
        let rows = sqlx::query(
            "SELECT role, content, name, tool_calls, tool_call_id FROM messages 
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
            let name: Option<String> = row.get(2);
            let tool_calls_str: Option<String> = row.get(3);
            let tool_call_id: Option<String> = row.get(4);

            let role = match role_str.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                "tool" => MessageRole::Tool,
                _ => MessageRole::User, // Default fallback
            };

            let tool_calls = if let Some(s) = tool_calls_str {
                if !s.is_empty() {
                    Some(serde_json::from_str::<Vec<ToolCall>>(&s).unwrap_or_default())
                } else {
                    None
                }
            } else {
                None
            };

            messages.push(ChatMessage {
                role,
                content,
                name,
                tool_calls,
                tool_call_id,
                reasoning_content: None,
            });
        }

        Ok(messages)
    }

    /// Clear old messages from a conversation while keeping the conversation record
    /// This prevents data loss when the same conversation_id is reused later
    /// Optionally keeps the most recent N messages for context
    pub async fn clear_conversation(
        &self,
        conversation_id: &str,
        keep_recent: Option<usize>,
    ) -> Result<()> {
        if let Some(keep_count) = keep_recent {
            // Keep the most recent N messages, delete the rest
            // Find the minimum created_at timestamp of messages we want to keep (the Nth most recent)
            // Then delete all messages with created_at less than that
            // This is more reliable than using IDs since it's based on actual timestamps
            let min_timestamp: Option<i64> = sqlx::query_scalar(
                "SELECT MIN(created_at) FROM (
                    SELECT created_at FROM messages 
                    WHERE conversation_id = ?1 
                    ORDER BY created_at DESC 
                    LIMIT ?2
                )",
            )
            .bind(conversation_id)
            .bind(keep_count as i64)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to find minimum timestamp of messages to keep")?;

            if let Some(min_timestamp_to_keep) = min_timestamp {
                // Delete all messages older than the oldest message we want to keep
                sqlx::query(
                    "DELETE FROM messages 
                     WHERE conversation_id = ?1 
                     AND created_at < ?2",
                )
                .bind(conversation_id)
                .bind(min_timestamp_to_keep)
                .execute(&self.pool)
                .await
                .context("Failed to clear old messages from conversation")?;
            }
            // If min_timestamp is None, there are no messages or fewer than keep_count, so nothing to delete
        } else {
            // Delete all messages but keep the conversation record
            sqlx::query("DELETE FROM messages WHERE conversation_id = ?1")
                .bind(conversation_id)
                .execute(&self.pool)
                .await
                .context("Failed to clear messages from conversation")?;
        }

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
    /// Get the last message ID (for rollback purposes)
    pub async fn get_last_message_id(&self) -> Result<i64> {
        let id: Option<i64> = sqlx::query_scalar("SELECT MAX(id) FROM messages")
            .fetch_optional(&self.pool)
            .await
            .context("Failed to get last message ID")?;

        Ok(id.unwrap_or(0))
    }

    /// Delete messages after a specific ID (rollback)
    pub async fn delete_messages_after_id(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE id > ?1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete messages")?;

        Ok(())
    }
}
