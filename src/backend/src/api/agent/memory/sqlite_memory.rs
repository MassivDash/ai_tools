use crate::api::agent::core::types::{ChatMessage, Conversation, MessageRole, ToolCall};
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
                    "⚠️  Detected outdated schema (messages). Resetting tables (Development Mode)..."
                );
                sqlx::query("DROP TABLE messages")
                    .execute(&pool)
                    .await
                    .context("Failed to drop old messages table")?;
            }
        }

        // Check if conversations table has title and model
        let conv_table_exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='conversations'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if conv_table_exists.is_some() {
            let has_title: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM pragma_table_info('conversations') WHERE name='title'",
            )
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            let has_model: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM pragma_table_info('conversations') WHERE name='model'",
            )
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            if has_title.is_none() || has_model.is_none() {
                println!("⚠️  Detected outdated schema (conversations). Resetting tables (Development Mode)...");
                // Need to drop messages first due to FK constraint
                sqlx::query("DROP TABLE IF EXISTS messages")
                    .execute(&pool)
                    .await
                    .context("Failed to drop messages table for reset")?;
                sqlx::query("DROP TABLE conversations")
                    .execute(&pool)
                    .await
                    .context("Failed to drop old conversations table")?;
            }
        }

        // Create tables
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT,
                model TEXT,
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
        model: Option<&str>,
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
                sqlx::query("INSERT INTO conversations (id, model) VALUES (?1, ?2)")
                    .bind(&id)
                    .bind(model)
                    .execute(&self.pool)
                    .await
                    .context("Failed to create conversation")?;
            } else if let Some(m) = model {
                // Update model if provided and conversation exists (implicit update)
                let _ = self.update_conversation_model(&id, m).await;
            }

            Ok(id)
        } else {
            // Generate a new conversation ID
            use uuid::Uuid;
            let id = Uuid::new_v4().to_string();

            // Create with default timestamp-based title
            // "Chat" + date/time from SQLite's datetime('now', 'localtime')
            sqlx::query(
                "INSERT INTO conversations (id, title, model) 
                 VALUES (?1, 'Chat ' || datetime('now', 'localtime'), ?2)",
            )
            .bind(&id)
            .bind(model)
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

        // Serialize content: Raw string for Text, JSON for Parts
        use crate::api::agent::core::types::MessageContent;
        let content_str = match &message.content {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Parts(parts) => serde_json::to_string(parts).unwrap_or_default(),
        };

        sqlx::query(
            "INSERT INTO messages (conversation_id, role, content, name, tool_calls, tool_call_id) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(conversation_id)
        .bind(role_str)
        .bind(&content_str)
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
            let content_str: String = row.get(1);
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

            use crate::api::agent::core::types::{ContentPart, MessageContent};
            // Deserialize content
            let content = if content_str.trim().starts_with('[') {
                match serde_json::from_str::<Vec<ContentPart>>(&content_str) {
                    Ok(parts) => MessageContent::Parts(parts),
                    Err(_) => MessageContent::Text(content_str), // Fallback to raw text if parse fails
                }
            } else {
                MessageContent::Text(content_str)
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

    /// Get all conversations
    pub async fn get_conversations(&self) -> Result<Vec<Conversation>> {
        let rows = sqlx::query(
            "SELECT id, title, model, created_at FROM conversations ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch conversations")?;

        let mut conversations = Vec::new();
        for row in rows {
            let model: Option<String> = row.get(2);
            // println!("DEBUG: Fetching conversation: {:?}, model: {:?}", row.get::<String, _>(0), model);
            conversations.push(Conversation {
                id: row.get(0),
                title: row.get(1),
                model,
                created_at: row.get(3),
            });
        }

        // println!("DEBUG: Returning {} conversations", conversations.len());
        Ok(conversations)
    }

    /// Delete a conversation
    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<()> {
        // Messages are deleted via CASCADE, but we can be explicit if needed.
        // With ON DELETE CASCADE defined in schema, deleting conversation is enough.
        sqlx::query("DELETE FROM conversations WHERE id = ?1")
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete conversation")?;

        Ok(())
    }

    /// Update conversation title
    pub async fn update_conversation_title(
        &self,
        conversation_id: &str,
        title: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE conversations SET title = ?1 WHERE id = ?2")
            .bind(title)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .context("Failed to update conversation title")?;

        Ok(())
    }

    /// Update conversation model
    pub async fn update_conversation_model(
        &self,
        conversation_id: &str,
        model: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE conversations SET model = ?1 WHERE id = ?2")
            .bind(model)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .context("Failed to update conversation model")?;

        Ok(())
    }

    /// Get conversation title
    pub async fn get_title(&self, conversation_id: &str) -> Result<String> {
        let title: Option<String> =
            sqlx::query_scalar("SELECT title FROM conversations WHERE id = ?1")
                .bind(conversation_id)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to get conversation title")?;

        Ok(title.unwrap_or_else(|| "New Conversation".to_string()))
    }

    /// Drop both tables so that subsequent queries fail. Used to exercise the
    /// error paths of the handlers that sit on top of this store.
    #[cfg(test)]
    pub(crate) async fn drop_tables_for_tests(&self) {
        for statement in ["DROP TABLE messages", "DROP TABLE conversations"] {
            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .expect("Failed to drop table");
        }
    }
}

/// Build a `SqliteConversationMemory` backed by a throwaway on-disk SQLite file.
///
/// `SqliteConversationMemory::new` takes a filesystem path (it canonicalizes it
/// and creates the parent directory), so a `sqlite::memory:` URL is not an
/// option here. The returned `TempDir` must be kept alive for as long as the
/// store is used.
#[cfg(test)]
pub(crate) async fn new_test_memory() -> (tempfile::TempDir, SqliteConversationMemory) {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let memory = SqliteConversationMemory::new(dir.path().join("conversations.db"))
        .await
        .expect("Failed to initialize conversation memory");
    (dir, memory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::{ContentPart, FunctionCall, ImageUrl, MessageContent};

    fn user_message(text: &str) -> ChatMessage {
        ChatMessage {
            role: MessageRole::User,
            content: MessageContent::Text(text.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    }

    #[tokio::test]
    async fn test_new_creates_an_empty_store() {
        let (_dir, memory) = new_test_memory().await;

        assert!(memory
            .get_conversations()
            .await
            .expect("Failed to list conversations")
            .is_empty());
        assert_eq!(
            memory
                .get_last_message_id()
                .await
                .expect("Failed to get last id"),
            0
        );
    }

    #[tokio::test]
    async fn test_new_creates_the_parent_directory() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let nested = dir.path().join("deeply/nested/conversations.db");

        let memory = SqliteConversationMemory::new(&nested)
            .await
            .expect("Failed to initialize conversation memory");

        assert!(
            nested.exists(),
            "the database file should have been created"
        );
        assert!(memory.get_conversations().await.is_ok());
    }

    #[tokio::test]
    async fn test_reopening_an_existing_database_keeps_its_data() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("conversations.db");

        let id = {
            let memory = SqliteConversationMemory::new(&path)
                .await
                .expect("Failed to initialize");
            let id = memory
                .get_or_create_conversation_id(None, Some("model-a"))
                .await
                .expect("Failed to create conversation");
            memory
                .add_message(&id, user_message("hello"))
                .await
                .expect("Failed to add message");
            id
        };

        // The path now exists, which takes the `canonicalize` branch in `new`
        let reopened = SqliteConversationMemory::new(&path)
            .await
            .expect("Failed to reopen");

        assert_eq!(
            reopened
                .message_count(&id)
                .await
                .expect("Failed to count messages"),
            1
        );
        let conversations = reopened
            .get_conversations()
            .await
            .expect("Failed to list conversations");
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].model.as_deref(), Some("model-a"));
    }

    #[tokio::test]
    async fn test_new_resets_a_messages_table_without_tool_call_columns() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("legacy.db");

        // Build a legacy schema: `messages` without the `tool_calls` column.
        {
            let options = SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true);
            let pool = SqlitePool::connect_with(options)
                .await
                .expect("Failed to connect");
            sqlx::query(
                "CREATE TABLE conversations (id TEXT PRIMARY KEY, title TEXT, model TEXT, created_at INTEGER NOT NULL DEFAULT 0)",
            )
            .execute(&pool)
            .await
            .expect("Failed to create legacy conversations table");
            sqlx::query(
                "CREATE TABLE messages (id INTEGER PRIMARY KEY AUTOINCREMENT, conversation_id TEXT NOT NULL, role TEXT NOT NULL, content TEXT NOT NULL)",
            )
            .execute(&pool)
            .await
            .expect("Failed to create legacy messages table");
            sqlx::query("INSERT INTO conversations (id) VALUES ('legacy')")
                .execute(&pool)
                .await
                .expect("Failed to seed conversation");
            sqlx::query(
                "INSERT INTO messages (conversation_id, role, content) VALUES ('legacy', 'user', 'old')",
            )
            .execute(&pool)
            .await
            .expect("Failed to seed message");
            pool.close().await;
        }

        let memory = SqliteConversationMemory::new(&path)
            .await
            .expect("Failed to migrate legacy database");

        // Messages were dropped and recreated, conversations were preserved.
        assert_eq!(
            memory
                .message_count("legacy")
                .await
                .expect("Failed to count messages"),
            0
        );
        assert_eq!(
            memory
                .get_conversations()
                .await
                .expect("Failed to list conversations")
                .len(),
            1
        );

        // The recreated table accepts the current message shape.
        memory
            .add_message("legacy", user_message("new"))
            .await
            .expect("Failed to add message to migrated table");
    }

    #[tokio::test]
    async fn test_new_resets_a_conversations_table_without_title_or_model() {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("legacy-conversations.db");

        {
            let options = SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true);
            let pool = SqlitePool::connect_with(options)
                .await
                .expect("Failed to connect");
            sqlx::query(
                "CREATE TABLE conversations (id TEXT PRIMARY KEY, created_at INTEGER NOT NULL DEFAULT 0)",
            )
            .execute(&pool)
            .await
            .expect("Failed to create legacy conversations table");
            sqlx::query("INSERT INTO conversations (id) VALUES ('legacy')")
                .execute(&pool)
                .await
                .expect("Failed to seed conversation");
            pool.close().await;
        }

        let memory = SqliteConversationMemory::new(&path)
            .await
            .expect("Failed to migrate legacy database");

        assert!(
            memory
                .get_conversations()
                .await
                .expect("Failed to list conversations")
                .is_empty(),
            "the outdated conversations table should have been recreated"
        );
    }

    #[tokio::test]
    async fn test_get_or_create_conversation_id_generates_a_titled_conversation() {
        let (_dir, memory) = new_test_memory().await;

        let id = memory
            .get_or_create_conversation_id(None, Some("model-a"))
            .await
            .expect("Failed to create conversation");

        assert_eq!(id.len(), 36, "expected a uuid, got {}", id);
        let conversations = memory
            .get_conversations()
            .await
            .expect("Failed to list conversations");
        assert_eq!(conversations.len(), 1);
        assert_eq!(conversations[0].id, id);
        assert_eq!(conversations[0].model.as_deref(), Some("model-a"));
        let title = conversations[0].title.clone().expect("expected a title");
        assert!(title.starts_with("Chat "), "unexpected title: {}", title);
        assert_eq!(
            memory.get_title(&id).await.expect("Failed to get title"),
            title
        );
        assert!(conversations[0].created_at > 0);
    }

    #[tokio::test]
    async fn test_get_or_create_conversation_id_inserts_unknown_ids_without_a_title() {
        let (_dir, memory) = new_test_memory().await;

        let id = memory
            .get_or_create_conversation_id(Some("supplied-id".to_string()), None)
            .await
            .expect("Failed to create conversation");

        assert_eq!(id, "supplied-id");
        let conversations = memory
            .get_conversations()
            .await
            .expect("Failed to list conversations");
        assert_eq!(conversations.len(), 1);
        assert!(conversations[0].title.is_none());
        assert!(conversations[0].model.is_none());
        // A row with a NULL title decodes to an empty string, so the
        // "New Conversation" fallback only applies to a missing conversation.
        assert_eq!(
            memory.get_title(&id).await.expect("Failed to get title"),
            ""
        );
    }

    #[tokio::test]
    async fn test_get_or_create_conversation_id_updates_the_model_of_a_known_conversation() {
        let (_dir, memory) = new_test_memory().await;

        let id = memory
            .get_or_create_conversation_id(None, Some("model-a"))
            .await
            .expect("Failed to create conversation");

        let same_id = memory
            .get_or_create_conversation_id(Some(id.clone()), Some("model-b"))
            .await
            .expect("Failed to reuse conversation");

        assert_eq!(same_id, id);
        let conversations = memory
            .get_conversations()
            .await
            .expect("Failed to list conversations");
        assert_eq!(conversations.len(), 1, "no duplicate conversation was made");
        assert_eq!(conversations[0].model.as_deref(), Some("model-b"));

        // Passing no model leaves the stored model alone
        memory
            .get_or_create_conversation_id(Some(id.clone()), None)
            .await
            .expect("Failed to reuse conversation");
        let conversations = memory
            .get_conversations()
            .await
            .expect("Failed to list conversations");
        assert_eq!(conversations[0].model.as_deref(), Some("model-b"));
    }

    #[tokio::test]
    async fn test_add_and_get_messages_round_trips_every_role_and_field() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");

        let assistant = ChatMessage {
            role: MessageRole::Assistant,
            content: MessageContent::Text(String::new()),
            name: None,
            tool_calls: Some(vec![ToolCall {
                id: "call_1".to_string(),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: "weather".to_string(),
                    arguments: "{\"city\":\"Rome\"}".to_string(),
                },
            }]),
            tool_call_id: None,
            reasoning_content: Some("dropped on the way to storage".to_string()),
        };
        let tool = ChatMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text("sunny".to_string()),
            name: Some("weather".to_string()),
            tool_calls: None,
            tool_call_id: Some("call_1".to_string()),
            reasoning_content: None,
        };
        let system = ChatMessage {
            role: MessageRole::System,
            content: MessageContent::Text("be nice".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        };

        for message in [
            system,
            user_message("what is the weather?"),
            assistant,
            tool,
        ] {
            memory
                .add_message(&id, message)
                .await
                .expect("Failed to add message");
        }

        let messages = memory
            .get_messages(&id)
            .await
            .expect("Failed to get messages");
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].role, MessageRole::System);
        assert_eq!(messages[1].role, MessageRole::User);
        assert_eq!(messages[1].content.text(), "what is the weather?");

        assert_eq!(messages[2].role, MessageRole::Assistant);
        let calls = messages[2]
            .tool_calls
            .as_ref()
            .expect("expected stored tool calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].function.name, "weather");
        assert!(
            messages[2].reasoning_content.is_none(),
            "reasoning_content is not persisted"
        );

        assert_eq!(messages[3].role, MessageRole::Tool);
        assert_eq!(messages[3].name.as_deref(), Some("weather"));
        assert_eq!(messages[3].tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(messages[3].content.text(), "sunny");
    }

    #[tokio::test]
    async fn test_multipart_content_round_trips_as_parts() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");

        memory
            .add_message(
                &id,
                ChatMessage {
                    role: MessageRole::User,
                    content: MessageContent::Parts(vec![
                        ContentPart::Text {
                            text: "look at this".to_string(),
                        },
                        ContentPart::ImageUrl {
                            image_url: ImageUrl {
                                url: "http://example.com/a.png".to_string(),
                            },
                        },
                    ]),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                },
            )
            .await
            .expect("Failed to add message");

        let messages = memory
            .get_messages(&id)
            .await
            .expect("Failed to get messages");
        match &messages[0].content {
            MessageContent::Parts(parts) => assert_eq!(parts.len(), 2),
            other => panic!("Expected parts, got {:?}", other),
        }
        assert_eq!(messages[0].content.text(), "look at this");
    }

    #[tokio::test]
    async fn test_get_messages_falls_back_to_text_for_unparsable_bracket_content() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");

        // Text that merely looks like a JSON array must survive as plain text,
        // and an unknown stored role falls back to `User`.
        sqlx::query(
            "INSERT INTO messages (conversation_id, role, content) VALUES (?1, 'wizard', '[not json')",
        )
        .bind(&id)
        .execute(&memory.pool)
        .await
        .expect("Failed to insert raw message");

        let messages = memory
            .get_messages(&id)
            .await
            .expect("Failed to get messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[0].content.text(), "[not json");
    }

    #[tokio::test]
    async fn test_get_messages_for_unknown_conversation_is_empty() {
        let (_dir, memory) = new_test_memory().await;

        assert!(memory
            .get_messages("nope")
            .await
            .expect("Failed to get messages")
            .is_empty());
        assert_eq!(
            memory
                .message_count("nope")
                .await
                .expect("Failed to count messages"),
            0
        );
    }

    #[tokio::test]
    async fn test_clear_conversation_deletes_all_messages_but_keeps_the_conversation() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");

        for text in ["one", "two"] {
            memory
                .add_message(&id, user_message(text))
                .await
                .expect("Failed to add message");
        }

        memory
            .clear_conversation(&id, None)
            .await
            .expect("Failed to clear conversation");

        assert_eq!(
            memory
                .message_count(&id)
                .await
                .expect("Failed to count messages"),
            0
        );
        assert_eq!(
            memory
                .get_conversations()
                .await
                .expect("Failed to list conversations")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn test_clear_conversation_keeping_recent_messages() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");

        // `keep_recent` works on the created_at timestamp, so the messages that
        // must survive are given a newer timestamp than the ones being dropped.
        for (text, created_at) in [("oldest", 100), ("older", 200), ("newest", 300)] {
            sqlx::query(
                "INSERT INTO messages (conversation_id, role, content, created_at) VALUES (?1, 'user', ?2, ?3)",
            )
            .bind(&id)
            .bind(text)
            .bind(created_at)
            .execute(&memory.pool)
            .await
            .expect("Failed to insert message");
        }

        memory
            .clear_conversation(&id, Some(2))
            .await
            .expect("Failed to clear conversation");

        let remaining: Vec<String> = memory
            .get_messages(&id)
            .await
            .expect("Failed to get messages")
            .iter()
            .map(|m| m.content.text())
            .collect();
        assert_eq!(remaining, vec!["older", "newest"]);
    }

    #[tokio::test]
    async fn test_clear_conversation_keeping_more_than_exists_deletes_nothing() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");
        memory
            .add_message(&id, user_message("keep me"))
            .await
            .expect("Failed to add message");

        memory
            .clear_conversation(&id, Some(10))
            .await
            .expect("Failed to clear conversation");

        assert_eq!(
            memory
                .message_count(&id)
                .await
                .expect("Failed to count messages"),
            1
        );

        // An empty conversation has no minimum timestamp, so nothing happens
        let empty = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");
        memory
            .clear_conversation(&empty, Some(3))
            .await
            .expect("Failed to clear empty conversation");
        assert_eq!(
            memory
                .message_count(&empty)
                .await
                .expect("Failed to count messages"),
            0
        );
    }

    #[tokio::test]
    async fn test_rollback_via_last_message_id_and_delete_after() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");

        memory
            .add_message(&id, user_message("keep"))
            .await
            .expect("Failed to add message");
        let checkpoint = memory
            .get_last_message_id()
            .await
            .expect("Failed to get last id");
        assert_eq!(checkpoint, 1);

        for text in ["discard 1", "discard 2"] {
            memory
                .add_message(&id, user_message(text))
                .await
                .expect("Failed to add message");
        }
        assert_eq!(
            memory
                .get_last_message_id()
                .await
                .expect("Failed to get last id"),
            3
        );

        memory
            .delete_messages_after_id(checkpoint)
            .await
            .expect("Failed to roll back");

        let messages = memory
            .get_messages(&id)
            .await
            .expect("Failed to get messages");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content.text(), "keep");
    }

    #[tokio::test]
    async fn test_get_conversations_is_ordered_newest_first() {
        let (_dir, memory) = new_test_memory().await;

        for (id, created_at) in [("old", 100), ("new", 300), ("middle", 200)] {
            sqlx::query("INSERT INTO conversations (id, title, created_at) VALUES (?1, ?1, ?2)")
                .bind(id)
                .bind(created_at)
                .execute(&memory.pool)
                .await
                .expect("Failed to insert conversation");
        }

        let ids: Vec<String> = memory
            .get_conversations()
            .await
            .expect("Failed to list conversations")
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, vec!["new", "middle", "old"]);
    }

    #[tokio::test]
    async fn test_delete_conversation_cascades_to_its_messages() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");
        memory
            .add_message(&id, user_message("hello"))
            .await
            .expect("Failed to add message");

        memory
            .delete_conversation(&id)
            .await
            .expect("Failed to delete conversation");

        assert!(memory
            .get_conversations()
            .await
            .expect("Failed to list conversations")
            .is_empty());

        // Deleting an unknown conversation is a no-op rather than an error
        memory
            .delete_conversation("nope")
            .await
            .expect("Deleting a missing conversation should succeed");
    }

    #[tokio::test]
    async fn test_update_conversation_title_and_model() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");

        memory
            .update_conversation_title(&id, "Renamed chat")
            .await
            .expect("Failed to update title");
        memory
            .update_conversation_model(&id, "model-z")
            .await
            .expect("Failed to update model");

        assert_eq!(
            memory.get_title(&id).await.expect("Failed to get title"),
            "Renamed chat"
        );
        let conversations = memory
            .get_conversations()
            .await
            .expect("Failed to list conversations");
        assert_eq!(conversations[0].model.as_deref(), Some("model-z"));

        // Updating an unknown conversation affects nothing and does not error
        memory
            .update_conversation_title("nope", "ghost")
            .await
            .expect("Failed to update missing title");
        memory
            .update_conversation_model("nope", "ghost")
            .await
            .expect("Failed to update missing model");
        assert_eq!(
            memory
                .get_conversations()
                .await
                .expect("Failed to list conversations")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn test_get_title_of_unknown_conversation_falls_back() {
        let (_dir, memory) = new_test_memory().await;

        assert_eq!(
            memory.get_title("nope").await.expect("Failed to get title"),
            "New Conversation"
        );
    }

    #[tokio::test]
    async fn test_every_query_reports_an_error_once_the_tables_are_gone() {
        let (_dir, memory) = new_test_memory().await;
        let id = memory
            .get_or_create_conversation_id(None, None)
            .await
            .expect("Failed to create conversation");
        memory.drop_tables_for_tests().await;

        assert!(memory.get_conversations().await.is_err());
        assert!(memory.get_messages(&id).await.is_err());
        assert!(memory.message_count(&id).await.is_err());
        assert!(memory.get_last_message_id().await.is_err());
        assert!(memory.delete_messages_after_id(0).await.is_err());
        assert!(memory.clear_conversation(&id, None).await.is_err());
        assert!(memory.clear_conversation(&id, Some(1)).await.is_err());
        assert!(memory.add_message(&id, user_message("x")).await.is_err());
        assert!(memory.delete_conversation(&id).await.is_err());
        assert!(memory.update_conversation_title(&id, "t").await.is_err());
        assert!(memory.update_conversation_model(&id, "m").await.is_err());
        assert!(memory.get_title(&id).await.is_err());
        assert!(memory
            .get_or_create_conversation_id(Some(id), None)
            .await
            .is_err());
        assert!(memory
            .get_or_create_conversation_id(None, None)
            .await
            .is_err());
    }
}
