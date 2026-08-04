use crate::api::agent::core::types::MessageRole;
use crate::api::agent::memory::sqlite_memory::SqliteConversationMemory;
use crate::api::agent::service::utils::clean_response;
use reqwest::Client;
use std::sync::Arc;

/// Helper to attempt auto-naming the conversation
pub async fn attempt_conversation_naming(
    client: Client,
    llama_url: String,
    model_name: String,
    sqlite_memory: Arc<SqliteConversationMemory>,
    conversation_id: String,
) {
    // Check message count - only rename if it's new (e.g. 2 user/assistant messages)
    let count = match sqlite_memory.message_count(&conversation_id).await {
        Ok(c) => c,
        Err(e) => {
            println!("⚠️ [Naming] Failed to get message count: {}", e);
            return;
        }
    };

    println!(
        "🔍 [Naming] Conversation {} has {} messages",
        conversation_id, count
    );

    // Get current title to see if it's already been renamed
    let current_title = match sqlite_memory.get_title(&conversation_id).await {
        Ok(t) => t,
        Err(e) => {
            println!("⚠️ [Naming] Failed to get title: {}", e);
            return;
        }
    };

    // Check if title is still default "Chat ..." or "New Conversation"
    // If it doesn't start with "Chat " and isn't "New Conversation", it's likely been renamed by user or previous run.
    if !current_title.starts_with("Chat ") && current_title != "New Conversation" {
        println!(
            "ℹ️ [Naming] Skipping naming: conversation already named '{}'",
            current_title
        );
        return;
    }

    // We only want to rename early in the conversation, but allow for some buffer
    // Lower bound: need at least 2 messages for context
    // Upper bound: protect against massive context window costs, but relax it significantly (e.g. 50)
    if count < 2 {
        return;
    }
    if count > 50 {
        println!(
            "ℹ️ [Naming] Skipping naming: message count {} too high (limit 50)",
            count
        );
        return;
    }

    // Delay a bit to let the LLM server finish processing the previous request
    // Large models might be slow to release resources/slots
    tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;

    // Also check if title is still default "Chat ..." or "New Conversation" to avoid overwriting user rename.
    // Ideally we should check this, but for now we assume if count is low it hasn't been renamed manually yet.

    // Get messages to prompt for title
    let messages = match sqlite_memory.get_messages(&conversation_id).await {
        Ok(m) => m,
        Err(e) => {
            println!("⚠️ [Naming] Failed to get messages: {}", e);
            return;
        }
    };

    if messages.is_empty() {
        println!("ℹ️ [Naming] Skipping naming: no messages found");
        return;
    }

    // Construct prompt
    // We use the first user message + assistant response for context
    let context_msgs: Vec<String> = messages
        .iter()
        .filter(|m| m.role == MessageRole::User || m.role == MessageRole::Assistant)
        .take(2)
        .map(|m| {
            format!(
                "{}: {}",
                if m.role == MessageRole::User {
                    "User"
                } else {
                    "Assistant"
                },
                m.content.text()
            )
        })
        .collect();

    let context = context_msgs.join("\n");

    let prompt = format!(
        "Please provide a very short, concise title (max 5 words) for the following conversation. The title should summarize the topic. Return ONLY the title text, no quotes, no prefixes.\n\nConversation:\n{}", 
        context
    );

    // Call LLM for title
    // We use a simple non-streaming request
    let request = serde_json::json!({
        "model": model_name,
        "messages": [
            { "role": "user", "content": prompt }
        ],
        "temperature": 0.7,
        "max_tokens": 1000
    });

    println!(
        "📤 [Naming] Sending request to LLM (model: {})...",
        model_name
    );

    // Fire and forget-ish
    let res = match client
        .post(&llama_url)
        .json(&request)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("⚠️ [Naming] Failed to request title summary: {}", e);
            return;
        }
    };

    let status = res.status();
    if !status.is_success() {
        let text = res.text().await.unwrap_or_default();
        println!("⚠️ [Naming] LLM server error (status {}): {}", status, text);
        return;
    }

    if let Ok(json) = res.json::<serde_json::Value>().await {
        if let Some(content) = json["choices"][0]["message"]["content"].as_str() {
            let title = clean_response(content).replace("\"", "").trim().to_string();
            if !title.is_empty() {
                println!(
                    "📝 Auto-renaming conversation {} to '{}'",
                    conversation_id, title
                );
                let _ = sqlite_memory
                    .update_conversation_title(&conversation_id, &title)
                    .await;
            }
        } else {
            println!(
                "⚠️ [Naming] Unexpected JSON response structure (missing content): {:?}",
                json
            );
        }
    } else {
        println!("⚠️ [Naming] Failed to parse JSON response");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::{ChatMessage, MessageContent};
    use crate::api::agent::memory::sqlite_memory::new_test_memory;
    use crate::test_support::{MockLlm, MockLlmConfig, UNREACHABLE_LLM_URL};

    fn message(role: MessageRole, text: &str) -> ChatMessage {
        ChatMessage {
            role,
            content: MessageContent::Text(text.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    }

    /// A conversation carrying `messages`, with the auto-generated "Chat <date>"
    /// title that marks it as never renamed.
    async fn conversation_with(
        messages: Vec<ChatMessage>,
    ) -> (tempfile::TempDir, Arc<SqliteConversationMemory>, String) {
        let (dir, memory) = new_test_memory().await;
        let memory = Arc::new(memory);
        let id = memory
            .get_or_create_conversation_id(None, Some("test-model"))
            .await
            .expect("Failed to create conversation");
        for message in messages {
            memory
                .add_message(&id, message)
                .await
                .expect("Failed to store message");
        }
        (dir, memory, id)
    }

    fn exchange() -> Vec<ChatMessage> {
        vec![
            message(MessageRole::System, "you are helpful"),
            message(MessageRole::User, "what is the capital of Italy?"),
            message(MessageRole::Assistant, "Rome."),
            message(MessageRole::User, "and of France?"),
        ]
    }

    async fn name_against(
        url: &str,
        memory: &Arc<SqliteConversationMemory>,
        conversation_id: &str,
    ) -> String {
        attempt_conversation_naming(
            Client::new(),
            url.to_string(),
            "test-model".to_string(),
            Arc::clone(memory),
            conversation_id.to_string(),
        )
        .await;
        memory
            .get_title(conversation_id)
            .await
            .expect("Failed to read the title back")
    }

    #[tokio::test]
    async fn test_the_model_reply_becomes_the_conversation_title() {
        let llm = MockLlm::start(MockLlmConfig::replying("  \"Italy trip planning\"\n")).await;
        let (_dir, memory, id) = conversation_with(exchange()).await;

        let title = name_against(&llm.chat_url(), &memory, &id).await;

        // Quotes and surrounding whitespace are stripped before storing
        assert_eq!(title, "Italy trip planning");

        // Only the first user/assistant pair is used as context, and system turns
        // are left out entirely
        let requests = llm.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["model"], "test-model");
        let prompt = requests[0]["messages"][0]["content"]
            .as_str()
            .expect("the prompt should be a string");
        assert!(
            prompt.contains("User: what is the capital of Italy?"),
            "{}",
            prompt
        );
        assert!(prompt.contains("Assistant: Rome."), "{}", prompt);
        assert!(!prompt.contains("you are helpful"), "{}", prompt);
        assert!(!prompt.contains("and of France?"), "{}", prompt);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_conversation_the_user_already_named_is_left_alone() {
        let llm = MockLlm::start(MockLlmConfig::replying("A generated title")).await;
        let (_dir, memory, id) = conversation_with(exchange()).await;
        memory
            .update_conversation_title(&id, "My own title")
            .await
            .expect("Failed to rename");

        let title = name_against(&llm.chat_url(), &memory, &id).await;

        assert_eq!(title, "My own title");
        assert_eq!(llm.call_count(), 0, "the model must not be consulted");

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_conversation_with_a_single_message_is_left_alone() {
        let llm = MockLlm::start(MockLlmConfig::replying("A generated title")).await;
        let (_dir, memory, id) = conversation_with(vec![message(MessageRole::User, "hello")]).await;
        let before = memory.get_title(&id).await.expect("Failed to read title");

        let title = name_against(&llm.chat_url(), &memory, &id).await;

        assert_eq!(title, before);
        assert!(title.starts_with("Chat "), "{}", title);
        assert_eq!(llm.call_count(), 0, "the model must not be consulted");

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_long_conversation_is_left_alone() {
        let llm = MockLlm::start(MockLlmConfig::replying("A generated title")).await;
        let long: Vec<ChatMessage> = (0..51)
            .map(|i| message(MessageRole::User, &format!("message {}", i)))
            .collect();
        let (_dir, memory, id) = conversation_with(long).await;
        let before = memory.get_title(&id).await.expect("Failed to read title");

        let title = name_against(&llm.chat_url(), &memory, &id).await;

        assert_eq!(title, before);
        assert_eq!(llm.call_count(), 0, "the model must not be consulted");

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_server_error_leaves_the_title_untouched() {
        let mut config = MockLlmConfig::replying("never read");
        config.chat_status = 500;
        let llm = MockLlm::start(config).await;
        let (_dir, memory, id) = conversation_with(exchange()).await;
        let before = memory.get_title(&id).await.expect("Failed to read title");

        let title = name_against(&llm.chat_url(), &memory, &id).await;

        assert_eq!(title, before);
        assert_eq!(llm.call_count(), 1, "the request was in fact attempted");

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_an_unparsable_body_leaves_the_title_untouched() {
        let mut config = MockLlmConfig::replying("ignored");
        config.chat_body = "<html>not json</html>".to_string();
        let llm = MockLlm::start(config).await;
        let (_dir, memory, id) = conversation_with(exchange()).await;
        let before = memory.get_title(&id).await.expect("Failed to read title");

        let title = name_against(&llm.chat_url(), &memory, &id).await;

        assert_eq!(title, before);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_response_without_content_leaves_the_title_untouched() {
        let mut config = MockLlmConfig::replying("ignored");
        config.chat_body = serde_json::json!({ "choices": [] }).to_string();
        let llm = MockLlm::start(config).await;
        let (_dir, memory, id) = conversation_with(exchange()).await;
        let before = memory.get_title(&id).await.expect("Failed to read title");

        let title = name_against(&llm.chat_url(), &memory, &id).await;

        assert_eq!(title, before);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_blank_title_is_not_stored() {
        let llm = MockLlm::start(MockLlmConfig::replying("  \"\"  ")).await;
        let (_dir, memory, id) = conversation_with(exchange()).await;
        let before = memory.get_title(&id).await.expect("Failed to read title");

        let title = name_against(&llm.chat_url(), &memory, &id).await;

        assert_eq!(title, before);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_an_unreachable_server_leaves_the_title_untouched() {
        let (_dir, memory, id) = conversation_with(exchange()).await;
        let before = memory.get_title(&id).await.expect("Failed to read title");

        let title = name_against(
            &format!("{}/v1/chat/completions", UNREACHABLE_LLM_URL),
            &memory,
            &id,
        )
        .await;

        assert_eq!(title, before);
    }

    #[tokio::test]
    async fn test_a_store_without_tables_is_reported_and_skipped() {
        let llm = MockLlm::start(MockLlmConfig::replying("A generated title")).await;
        let (_dir, memory, id) = conversation_with(exchange()).await;
        memory.drop_tables_for_tests().await;

        // The message count lookup now fails, so the naming attempt gives up before
        // reaching the model instead of propagating the error.
        attempt_conversation_naming(
            Client::new(),
            llm.chat_url(),
            "test-model".to_string(),
            Arc::clone(&memory),
            id,
        )
        .await;

        assert_eq!(llm.call_count(), 0, "the model must not be consulted");

        llm.stop().await;
    }
}
