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
