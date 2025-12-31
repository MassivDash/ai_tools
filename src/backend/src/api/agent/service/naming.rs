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
        Err(_) => return,
    };

    // We only want to rename early in the conversation
    // Depending on when this is called (during or after), count might vary.
    // If called after response is stored, count should be >= 2.
    // Let's safe guard: if count is between 2 and 4.
    if !(2..=4).contains(&count) {
        return;
    }

    // Also check if title is still default "Chat ..." or "New Conversation" to avoid overwriting user rename.
    // Ideally we should check this, but for now we assume if count is low it hasn't been renamed manually yet.

    // Get messages to prompt for title
    let messages = match sqlite_memory.get_messages(&conversation_id).await {
        Ok(m) => m,
        Err(_) => return,
    };

    if messages.is_empty() {
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
        "Generate a very short, concise title (max 5 words) for this conversation based on the start. Do not use quotes or prefixes. Just the title.\n\nConversation:\n{}\n\nTitle:", 
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
        "max_tokens": 20
    });

    // Fire and forget-ish
    let res = match client.post(&llama_url).json(&request).send().await {
        Ok(r) => r,
        Err(e) => {
            println!("⚠️ Failed to request title summary: {}", e);
            return;
        }
    };

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
        }
    }
}
