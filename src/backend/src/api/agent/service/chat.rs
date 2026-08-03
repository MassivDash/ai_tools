use crate::api::agent::core::agent_loop::{execute_agent_loop, AgentLoopConfig};
use crate::api::agent::core::streaming::execute_agent_loop_streaming;
use crate::api::agent::core::types::{
    ActiveGenerations, AgentChatRequest, AgentChatResponse, AgentConfig, AgentStreamEvent,
    ChatMessage, MessageContent, MessageRole,
};
use crate::api::agent::memory::sqlite_memory::SqliteConversationMemory;
use crate::api::agent::service::naming::attempt_conversation_naming;
use crate::api::agent::service::utils::clean_response;
use crate::api::agent::service::websocket::AgentWebSocketState;
use crate::api::agent::tools::{
    self,
    framework::{registry::ToolRegistry, selector::ToolSelector},
};
use crate::api::llama_server::types::Config;
use crate::api::pageindex::storage::PageIndexStorage;
use actix_web::{post, web, HttpResponse, Responder, Result as ActixResult};
use futures::StreamExt;
use reqwest::Client;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Chat completion endpoint
#[post("/api/agent/chat")]
pub async fn agent_chat(
    req: web::Json<AgentChatRequest>,
    agent_config: web::Data<Arc<Mutex<AgentConfig>>>,
    chroma_address: web::Data<String>,
    _chromadb_config: web::Data<Arc<Mutex<crate::api::chromadb::config::types::ChromaDBConfig>>>,
    llama_config: web::Data<Arc<Mutex<Config>>>,
    sqlite_memory: web::Data<Arc<SqliteConversationMemory>>,
    pageindex_storage: web::Data<Arc<PageIndexStorage>>,
) -> ActixResult<HttpResponse> {
    let config = agent_config.lock().unwrap().clone();

    // Get model name from llama_server config
    let model_name = {
        let llama_config_guard = llama_config.lock().unwrap();
        llama_config_guard.hf_model.clone()
    };

    // Construct Llama URL from config
    let (llama_host, llama_port) = {
        let llama_config_guard = llama_config.lock().unwrap();
        (
            llama_config_guard
                .host
                .clone()
                .unwrap_or_else(|| "localhost".to_string()),
            llama_config_guard.port.unwrap_or(8090),
        )
    };

    let host_for_url = if llama_host == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        llama_host
    };
    let llama_base_url = format!("http://{}:{}", host_for_url, llama_port);

    // Get or create conversation ID from SQLite
    let conversation_id = sqlite_memory
        .get_or_create_conversation_id(req.conversation_id.clone(), Some(&model_name))
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to get conversation ID: {}",
                e
            ))
        })?;

    // Build tool registry dynamically based on configuration
    let mut tool_registry = ToolRegistry::new();

    // Register ChromaDB tool if configured
    // Fetch available collections from ChromaDB
    let available_collections = if config
        .enabled_tools
        .contains(&crate::api::agent::core::types::ToolType::ChromaDB)
    {
        if let Ok(client) =
            crate::api::chromadb::client::ChromaDBClient::new(chroma_address.as_str())
        {
            client.list_collections().await.unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let available_page_indexes = pageindex_storage.list_summaries().await.unwrap_or_default();

    let context = tools::RegisterContext {
        chroma_address: Some(chroma_address.as_str()),
        available_collections: &available_collections,
        available_page_indexes: &available_page_indexes,
    };
    tools::register_all(&mut tool_registry, &config, &context);

    // Wrap registry in Arc for sharing
    let tool_registry_arc = Arc::new(tool_registry);

    // Build tool definitions for OpenAI-compatible API
    let tools = tool_registry_arc.build_tool_definitions().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to build tool definitions: {}",
            e
        ))
    })?;

    // Log tool registry stats and verify registration
    let tool_count = tool_registry_arc.count();
    let all_tool_ids = tool_registry_arc.get_all_tool_ids();
    println!(
        "📦 Tool registry: {} tool(s) registered: {:?}",
        tool_count, all_tool_ids
    );

    // Verify all tools are properly registered and accessible
    for tool_id in &all_tool_ids {
        if !tool_registry_arc.is_registered(tool_id) {
            println!(
                "⚠️ Warning: Tool {} marked as registered but not found in registry",
                tool_id
            );
        } else if let Some(tool) = tool_registry_arc.get_tool(tool_id) {
            // Tool exists, verify it's available
            if !tool.is_available() {
                println!("⚠️ Tool {} is registered but not available", tool_id);
            }
        }
    }

    // Get all tools and verify they're accessible
    let all_tools = tool_registry_arc.get_all_tools();
    for tool in &all_tools {
        // Verify tool is available (this uses the is_available method from the trait)
        if !tool.is_available() {
            println!("⚠️ Tool {} is not available", tool.metadata().name);
        }
    }

    // Create tool selector for intelligent tool selection
    let tool_selector = ToolSelector::new(Arc::clone(&tool_registry_arc));

    // Build system prompt using tool selector
    // The prompt already instructs the LLM when NOT to use tools (greetings, small talk, etc.)
    // The LLM will decide which tools to use based on the prompt
    let system_prompt = tool_selector.build_system_prompt();
    let system_prompt_clone = system_prompt.clone();

    // Get conversation history from SQLite (only user/assistant messages)
    let messages = sqlite_memory
        .get_messages(&conversation_id)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to get conversation history: {}",
                e
            ))
        })?;

    // Always start with fresh system prompt
    let mut messages_with_system = vec![ChatMessage {
        role: MessageRole::System,
        content: MessageContent::Text(system_prompt),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    }];

    // Add conversation history from SQLite
    messages_with_system.extend(messages);

    // Add current user message
    let new_message = if let Some(tool_result) = &req.tool_result {
        ChatMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(tool_result.result.clone()),
            name: Some(tool_result.tool_name.clone()),
            tool_calls: None,
            tool_call_id: tool_result.tool_call_id.clone(),
            reasoning_content: None,
        }
    } else {
        ChatMessage {
            role: MessageRole::User,
            content: req.message.clone(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    };
    messages_with_system.push(new_message.clone());

    // Store message
    sqlite_memory
        .add_message(&conversation_id, new_message)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to store message: {}", e))
        })?;

    let messages = messages_with_system;

    // Model update is now handled by get_or_create_conversation_id

    // Call llama.cpp server
    // Call llama.cpp server
    let llama_url = format!("{}/v1/chat/completions", llama_base_url);
    let client = Client::new();

    // Get conversation message count from SQLite
    let conversation_msg_count = sqlite_memory
        .message_count(&conversation_id)
        .await
        .unwrap_or(0);

    println!(
        "📊 Conversation history: {} messages, Tools available: {}",
        conversation_msg_count,
        tools.len()
    );
    if !tools.is_empty() {
        println!(
            "🔧 Available tools: {:?}",
            tools.iter().map(|t| &t.function.name).collect::<Vec<_>>()
        );
    }

    // Get last message ID before starting loop (for potential rollback)
    let last_message_id_before_loop = sqlite_memory.get_last_message_id().await.unwrap_or(0);

    // Execute agent loop - allows iterative tool use
    let loop_config = AgentLoopConfig {
        debug_logging: config.debug_logging,
        ..AgentLoopConfig::default()
    };
    let mut loop_result = execute_agent_loop(
        &client,
        &llama_url,
        model_name.clone(),
        messages.clone(),
        tools.clone(),
        tool_registry_arc.clone(),
        Arc::clone(&sqlite_memory),
        conversation_id.clone(),
        loop_config,
    )
    .await
    .map_err(|e| {
        println!("Agent loop error: {}", e);
        actix_web::error::ErrorInternalServerError(format!("Agent loop failed: {}", e))
    })?;

    // If agent got stuck, recover by restarting with clean context
    if loop_result.stuck {
        println!("🔄 Agent got stuck, attempting rollback and clean context recovery...");

        // Rollback: delete any messages created during the stuck loop
        if let Err(e) = sqlite_memory
            .delete_messages_after_id(last_message_id_before_loop)
            .await
        {
            println!("⚠️ Failed to rollback messages after stuck loop: {}", e);
        } else {
            println!(
                "✅ Rolled back messages to ID {}",
                last_message_id_before_loop
            );
        }

        // Get clean conversation history from SQLite (only user/assistant messages)
        let clean_messages = sqlite_memory
            .get_messages(&conversation_id)
            .await
            .map_err(|e| {
                actix_web::error::ErrorInternalServerError(format!(
                    "Failed to get clean conversation history: {}",
                    e
                ))
            })?;

        // Build fresh context with system prompt + conversation history
        let mut recovery_messages = vec![ChatMessage {
            role: MessageRole::System,
            content: MessageContent::Text(system_prompt_clone),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }];
        recovery_messages.extend(clean_messages);

        // Try again with clean context and reduced max iterations
        let recovery_config = AgentLoopConfig {
            max_iterations: 5, // Reduced for recovery attempt
            debug_logging: config.debug_logging,
            ..Default::default()
        };

        loop_result = execute_agent_loop(
            &client,
            &llama_url,
            model_name.clone(),
            recovery_messages,
            tools,
            tool_registry_arc,
            Arc::clone(&sqlite_memory),
            conversation_id.clone(),
            recovery_config,
        )
        .await
        .map_err(|e| {
            println!("Recovery attempt failed: {}", e);
            actix_web::error::ErrorInternalServerError(format!("Recovery failed: {}", e))
        })?;

        if loop_result.stuck {
            println!("⚠️ Recovery attempt also got stuck, returning partial response");
        }
    }

    // Clean the final message
    let final_message = clean_response(&loop_result.final_message);

    // Check conversation size and clear if too large (prevent database bloat)
    let msg_count = sqlite_memory
        .message_count(&conversation_id)
        .await
        .unwrap_or(0);

    // If conversation has more than 100 messages, clear old messages to prevent bloat
    // Keep the most recent 20 messages for context continuity
    if msg_count > 100 {
        println!(
            "🧹 Conversation {} has {} messages, clearing old messages (keeping last 20)",
            conversation_id, msg_count
        );
        if let Err(e) = sqlite_memory
            .clear_conversation(&conversation_id, Some(20))
            .await
        {
            println!("⚠️ Failed to clear old messages: {}", e);
        } else {
            println!(
                "✅ Cleared old messages from conversation {} (kept last 20)",
                conversation_id
            );
        }
    }

    println!(
        "✅ Agent loop completed after {} iterations",
        loop_result.iterations
    );

    let sqlite_memory_clone = sqlite_memory.get_ref().clone();
    let conversation_id_clone = conversation_id.clone();
    let client_clone = client.clone();
    let llama_url_clone = llama_url.to_string();
    let model_name_clone = model_name.clone();

    // Spawn background task for auto-naming (fire and forget)
    actix_rt::spawn(async move {
        attempt_conversation_naming(
            client_clone,
            llama_url_clone, // already formatted
            model_name_clone,
            sqlite_memory_clone,
            conversation_id_clone,
        )
        .await;
    });

    Ok(HttpResponse::Ok().json(AgentChatResponse {
        success: true,
        message: final_message,
        conversation_id: Some(conversation_id),
        tool_calls: if loop_result.tool_calls.is_empty() {
            None
        } else {
            Some(loop_result.tool_calls)
        },
    }))
}

/// Streaming chat completion endpoint using Server-Sent Events (SSE)
/// Also broadcasts events via WebSocket for real-time updates
#[post("/api/agent/chat/stream")]
#[allow(clippy::too_many_arguments)]
pub async fn agent_chat_stream(
    req: web::Json<AgentChatRequest>,
    agent_config: web::Data<Arc<Mutex<AgentConfig>>>,
    chroma_address: web::Data<String>,
    _chromadb_config: web::Data<Arc<Mutex<crate::api::chromadb::config::types::ChromaDBConfig>>>,
    llama_config: web::Data<Arc<Mutex<Config>>>,
    sqlite_memory: web::Data<Arc<SqliteConversationMemory>>,
    agent_ws_state: web::Data<Arc<AgentWebSocketState>>,
    active_generations: web::Data<ActiveGenerations>,
    pageindex_storage: web::Data<Arc<PageIndexStorage>>,
) -> ActixResult<HttpResponse> {
    let config = agent_config.lock().unwrap().clone();

    // Get model name from llama_server config
    let model_name = {
        let llama_config_guard = llama_config.lock().unwrap();
        llama_config_guard.hf_model.clone()
    };

    // Construct Llama URL from config
    let (llama_host, llama_port) = {
        let llama_config_guard = llama_config.lock().unwrap();
        (
            llama_config_guard
                .host
                .clone()
                .unwrap_or_else(|| "localhost".to_string()),
            llama_config_guard.port.unwrap_or(8090),
        )
    };
    let host_for_url = if llama_host == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        llama_host
    };
    let llama_base_url = format!("http://{}:{}", host_for_url, llama_port);

    // Get or create conversation ID
    let conversation_id = sqlite_memory
        .get_or_create_conversation_id(req.conversation_id.clone(), Some(&model_name))
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to get conversation ID: {}",
                e
            ))
        })?;

    // Build tool registry (same as non-streaming endpoint)
    let mut tool_registry = ToolRegistry::new();

    // Fetch available collections from ChromaDB
    let available_collections = if config
        .enabled_tools
        .contains(&crate::api::agent::core::types::ToolType::ChromaDB)
    {
        if let Ok(client) =
            crate::api::chromadb::client::ChromaDBClient::new(chroma_address.as_str())
        {
            client.list_collections().await.unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let available_page_indexes = pageindex_storage.list_summaries().await.unwrap_or_default();

    let context = tools::RegisterContext {
        chroma_address: Some(chroma_address.as_str()),
        available_collections: &available_collections,
        available_page_indexes: &available_page_indexes,
    };
    tools::register_all(&mut tool_registry, &config, &context);

    let tool_registry_arc = Arc::new(tool_registry);
    let tools = tool_registry_arc.build_tool_definitions().map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!(
            "Failed to build tool definitions: {}",
            e
        ))
    })?;

    let tool_selector = ToolSelector::new(Arc::clone(&tool_registry_arc));
    let system_prompt = tool_selector.build_system_prompt();

    // Get conversation history
    let messages = sqlite_memory
        .get_messages(&conversation_id)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to get conversation history: {}",
                e
            ))
        })?;

    let mut messages_with_system = vec![ChatMessage {
        role: MessageRole::System,
        content: MessageContent::Text(system_prompt),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    }];

    messages_with_system.extend(messages);

    let new_message = if let Some(tool_result) = &req.tool_result {
        ChatMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(tool_result.result.clone()),
            name: Some(tool_result.tool_name.clone()),
            tool_calls: None,
            tool_call_id: tool_result.tool_call_id.clone(),
            reasoning_content: None,
        }
    } else {
        ChatMessage {
            role: MessageRole::User,
            content: req.message.clone(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }
    };
    messages_with_system.push(new_message.clone());

    // Store message
    sqlite_memory
        .add_message(&conversation_id, new_message)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!("Failed to store message: {}", e))
        })?;

    // model_name is already retrieved above

    let llama_url = format!("{}/v1/chat/completions", llama_base_url);
    let client = Client::new();

    // Create channel for streaming events (SSE) (Bounded for backpressure)
    let (tx, rx) = mpsc::channel::<Result<AgentStreamEvent, anyhow::Error>>(100);

    // Create Cancellation Token using Watch Channel
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    // Register cancellation token
    {
        let mut map = active_generations.lock().unwrap();
        map.insert(conversation_id.clone(), cancel_tx);
    }

    // Clone necessary data for the streaming task
    let client_clone = client.clone();
    let llama_url_clone = llama_url.to_string();
    let model_name_clone = model_name.clone();
    let tools_clone = tools.clone();
    let tool_registry_clone = Arc::clone(&tool_registry_arc);
    let sqlite_memory_clone = sqlite_memory.get_ref().clone();
    let conversation_id_clone = conversation_id.clone();
    let agent_ws_state_clone = agent_ws_state.get_ref().clone();
    let loop_config = AgentLoopConfig {
        debug_logging: config.debug_logging,
        ..AgentLoopConfig::default()
    };
    let active_generations_clone = active_generations.get_ref().clone();

    // Spawn the agent loop in a background task
    actix_rt::spawn(async move {
        // Create a wrapper sender that broadcasts to both SSE and WebSocket (Bounded)
        let tx_sse = tx.clone();
        let agent_ws_broadcast = agent_ws_state_clone.clone();
        let (tx_wrapper, mut rx_wrapper) =
            mpsc::channel::<Result<AgentStreamEvent, anyhow::Error>>(100);

        // Spawn task to duplicate events to both SSE and WebSocket
        actix_rt::spawn(async move {
            while let Some(event_result) = rx_wrapper.recv().await {
                // Broadcast to WebSocket first (if successful)
                if let Ok(event) = &event_result {
                    agent_ws_broadcast.broadcast(event);
                }
                // Send to SSE (need to handle error case)
                // This await will block if SSE client is slow, or fail if disconnected
                match &event_result {
                    Ok(event) => {
                        if tx_sse.send(Ok(event.clone())).await.is_err() {
                            // Client disconnected logic handled by channel drop/backpressure usually,
                            // but explicit cancel is triggered by API now.
                            // If network disconnects, we could optionally trigger cancel too?
                            break;
                        }
                    }
                    Err(e) => {
                        if tx_sse.send(Err(anyhow::anyhow!("{}", e))).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // Execute streaming loop with cancellation support
        let result = execute_agent_loop_streaming(
            &client_clone,
            &llama_url_clone,
            model_name_clone.clone(),
            messages_with_system,
            tools_clone,
            tool_registry_clone,
            sqlite_memory_clone.clone(),
            conversation_id_clone.clone(),
            loop_config,
            tx_wrapper,
            cancel_rx, // Pass the watch receiver
        )
        .await;

        if let Err(e) = result {
            println!("Streaming agent loop error: {}", e);
        }

        // Cleanup cancellation token
        {
            let mut map = active_generations_clone.lock().unwrap();
            map.remove(&conversation_id_clone);
        }

        // Attempt naming after stream finishes
        attempt_conversation_naming(
            client_clone,
            llama_url_clone,
            model_name_clone,
            sqlite_memory_clone,
            conversation_id_clone,
        )
        .await;
    });

    // Convert events to SSE format
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(
        move |event_result| -> Result<web::Bytes, actix_web::Error> {
            match event_result {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                    Ok(web::Bytes::from(format!("data: {}\n\n", json)))
                }
                Err(e) => {
                    let error_event = AgentStreamEvent::Error {
                        message: format!("{:#}", e),
                    };
                    let json =
                        serde_json::to_string(&error_event).unwrap_or_else(|_| "{}".to_string());
                    Ok(web::Bytes::from(format!("data: {}\n\n", json)))
                }
            }
        },
    );

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .append_header(("Cache-Control", "no-cache"))
        .append_header(("Connection", "keep-alive"))
        .streaming(stream))
}

#[post("/api/agent/chat/{conversation_id}/cancel")]
pub async fn cancel_agent_generation(
    path: web::Path<String>,
    active_generations: web::Data<ActiveGenerations>,
) -> impl Responder {
    let conversation_id = path.into_inner();
    println!(
        "Received cancellation request for conversation {}",
        conversation_id
    );

    let map = active_generations.lock().unwrap();
    if let Some(tx) = map.get(&conversation_id) {
        let _ = tx.send(true); // Send cancellation signal
        HttpResponse::Ok().json(serde_json::json!({"status": "cancelled"}))
    } else {
        println!(
            "No active generation found for conversation {}",
            conversation_id
        );
        HttpResponse::NotFound().json(serde_json::json!({"error": "No active generation found"}))
    }
}

/// Tests for the non-streaming endpoints of this module. The SSE endpoint is
/// covered separately, since it reaches into `core::streaming`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::ToolType;
    use crate::api::agent::memory::sqlite_memory::new_test_memory;
    use crate::api::chromadb::config::types::ChromaDBConfig;
    use crate::test_support::{tool_call_completion, MockLlm, MockLlmConfig, UNREACHABLE_LLM_URL};
    use actix_web::http::StatusCode;
    use actix_web::{test, App};
    use std::collections::HashMap;

    /// A tool name that no registry in these tests knows about, so a model that
    /// asks for it only ever produces a "not found" tool result - nothing runs.
    const MISSING_TOOL: &str = "definitely_not_a_registered_tool";

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

    async fn memory_with(
        messages: Vec<(&str, ChatMessage)>,
    ) -> (tempfile::TempDir, Arc<SqliteConversationMemory>) {
        let (dir, memory) = new_test_memory().await;
        let memory = Arc::new(memory);
        for (conversation_id, message) in messages {
            memory
                .get_or_create_conversation_id(
                    Some(conversation_id.to_string()),
                    Some("test-model"),
                )
                .await
                .expect("Failed to create conversation");
            memory
                .add_message(conversation_id, message)
                .await
                .expect("Failed to store message");
        }
        (dir, memory)
    }

    /// Posts `payload` to the chat endpoint, wired to a llama server on
    /// `llama_host:llama_port`. ChromaDB is pointed at a dead address on purpose:
    /// no test here enables the ChromaDB tool, so it must never be contacted.
    async fn post_chat(
        llama_host: &str,
        llama_port: u16,
        memory: &Arc<SqliteConversationMemory>,
        enabled_tools: Vec<ToolType>,
        payload: serde_json::Value,
    ) -> (StatusCode, Vec<u8>) {
        let pageindex_storage = Arc::new(
            PageIndexStorage::new(":memory:")
                .await
                .expect("Failed to create page index storage"),
        );
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::new(Mutex::new(AgentConfig {
                    enabled_tools,
                    debug_logging: false,
                }))))
                .app_data(web::Data::new(UNREACHABLE_LLM_URL.to_string()))
                .app_data(web::Data::new(Arc::new(Mutex::new(
                    ChromaDBConfig::default(),
                ))))
                .app_data(web::Data::new(Arc::new(Mutex::new(Config {
                    hf_model: "test-model".to_string(),
                    host: Some(llama_host.to_string()),
                    port: Some(llama_port),
                    ..Default::default()
                }))))
                .app_data(web::Data::new(Arc::clone(memory)))
                .app_data(web::Data::new(pageindex_storage))
                .service(agent_chat),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/agent/chat")
            .set_json(payload)
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        let body = test::read_body(resp).await.to_vec();
        (status, body)
    }

    fn as_json(body: &[u8]) -> serde_json::Value {
        serde_json::from_slice(body).unwrap_or_else(|e| {
            panic!(
                "Body was not JSON ({}): {}",
                e,
                String::from_utf8_lossy(body)
            )
        })
    }

    /// The roles of the messages the model was sent on request `nth`.
    fn sent_roles(request: &serde_json::Value) -> Vec<String> {
        request["messages"]
            .as_array()
            .expect("messages should be an array")
            .iter()
            .map(|m| {
                m["role"]
                    .as_str()
                    .expect("role should be a string")
                    .to_string()
            })
            .collect()
    }

    #[actix_web::test]
    async fn test_chat_answers_and_persists_the_exchange() {
        let llm = MockLlm::start(MockLlmConfig::replying("Rome is the capital.")).await;
        let (_dir, memory) = memory_with(vec![]).await;

        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({ "message": "What is the capital of Italy?" }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let body = as_json(&body);
        assert_eq!(body["success"], true);
        assert_eq!(body["message"], "Rome is the capital.");
        assert!(body["tool_calls"].is_null(), "{}", body);

        // A conversation was created and both turns were stored against it
        let conversation_id = body["conversation_id"]
            .as_str()
            .expect("a conversation id should be returned")
            .to_string();
        let stored = memory
            .get_messages(&conversation_id)
            .await
            .expect("Failed to read the conversation back");
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].role, MessageRole::User);
        assert_eq!(stored[0].content.text(), "What is the capital of Italy?");
        assert_eq!(stored[1].role, MessageRole::Assistant);
        assert_eq!(stored[1].content.text(), "Rome is the capital.");

        // The model is given a system prompt ahead of the user turn, and no tools
        let requests = llm.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(sent_roles(&requests[0]), vec!["system", "user"]);
        assert_eq!(requests[0]["model"], "test-model");
        assert!(requests[0].get("tools").is_none(), "{}", requests[0]);

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_replays_the_history_of_a_known_conversation() {
        let llm = MockLlm::start(MockLlmConfig::replying("Paris.")).await;
        let (_dir, memory) = memory_with(vec![
            ("conv-1", message(MessageRole::User, "capital of Italy?")),
            ("conv-1", message(MessageRole::Assistant, "Rome.")),
        ])
        .await;

        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({ "message": "and of France?", "conversation_id": "conv-1" }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let body = as_json(&body);
        assert_eq!(body["conversation_id"], "conv-1");
        assert_eq!(body["message"], "Paris.");

        let requests = llm.requests();
        assert_eq!(
            sent_roles(&requests[0]),
            vec!["system", "user", "assistant", "user"]
        );
        assert_eq!(requests[0]["messages"][3]["content"], "and of France?");

        // The new turn and the answer are appended to the existing history
        let stored = memory
            .get_messages("conv-1")
            .await
            .expect("Failed to read the conversation back");
        assert_eq!(stored.len(), 4);
        assert_eq!(stored[3].content.text(), "Paris.");

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_accepts_a_wildcard_llama_host() {
        let llm = MockLlm::start(MockLlmConfig::replying("still reachable")).await;
        let (_dir, memory) = memory_with(vec![]).await;

        // 0.0.0.0 is a bind address, not a destination, so it is rewritten to loopback
        let (status, body) = post_chat(
            "0.0.0.0",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({ "message": "hi" }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(as_json(&body)["message"], "still reachable");

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_advertises_the_enabled_tools() {
        let llm = MockLlm::start(MockLlmConfig::replying("no tool needed")).await;
        let (_dir, memory) = memory_with(vec![]).await;

        // AskHuman is entirely local: its `execute` only ever bails out, and the
        // canned reply below never asks for it anyway.
        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![ToolType::AskHuman],
            serde_json::json!({ "message": "hi" }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(as_json(&body)["message"], "no tool needed");

        let requests = llm.requests();
        assert_eq!(requests[0]["tool_choice"], "auto");
        assert_eq!(requests[0]["tools"][0]["function"]["name"], "ask_human");
        // The system prompt describes the tool the agent may use
        let system = requests[0]["messages"][0]["content"]
            .as_str()
            .expect("the system prompt should be a string");
        assert!(system.contains("ask_human"), "{}", system);

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_stores_a_submitted_tool_result_as_a_tool_turn() {
        let llm = MockLlm::start(MockLlmConfig::replying("Thanks, noted.")).await;
        let (_dir, memory) = memory_with(vec![(
            "conv-1",
            message(MessageRole::User, "ask me something"),
        )])
        .await;

        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({
                "message": "",
                "conversation_id": "conv-1",
                "tool_result": {
                    "tool_call_id": "call_1",
                    "tool_name": "ask_human",
                    "result": "Option B"
                }
            }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(as_json(&body)["message"], "Thanks, noted.");

        let stored = memory
            .get_messages("conv-1")
            .await
            .expect("Failed to read the conversation back");
        assert_eq!(stored.len(), 3);
        assert_eq!(stored[1].role, MessageRole::Tool);
        assert_eq!(stored[1].name.as_deref(), Some("ask_human"));
        assert_eq!(stored[1].content.text(), "Option B");
        assert_eq!(stored[1].tool_call_id.as_deref(), Some("call_1"));

        // The tool turn reaches the model folded into a user message
        let sent = &llm.requests()[0]["messages"];
        assert_eq!(sent[2]["content"], "Tool results:\nask_human: Option B");

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_cleans_reasoning_out_of_the_reply() {
        let llm = MockLlm::start(MockLlmConfig::replying(
            "Thought: I should look this up\nAnswer: The result is 4",
        ))
        .await;
        let (_dir, memory) = memory_with(vec![]).await;

        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({ "message": "2 + 2?" }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let body = as_json(&body);
        assert_eq!(body["message"], "The result is 4");

        // Note: only the response is cleaned - the stored history keeps the raw text
        let stored = memory
            .get_messages(body["conversation_id"].as_str().expect("id"))
            .await
            .expect("Failed to read the conversation back");
        assert_eq!(
            stored[1].content.text(),
            "Thought: I should look this up\nAnswer: The result is 4"
        );

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_rolls_back_and_retries_a_stuck_agent() {
        // A model that only ever asks for a tool it cannot get: the loop hits its
        // iteration cap, the messages it wrote are rolled back, and it is retried
        // once with a clean context and a lower cap.
        let llm = MockLlm::start(MockLlmConfig::replying_with_bodies(vec![
            tool_call_completion("call_1", MISSING_TOOL, "{}"),
        ]))
        .await;
        let (_dir, memory) = memory_with(vec![]).await;

        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({ "message": "go in circles" }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        let body = as_json(&body);
        assert_eq!(
            body["message"],
            "I've gathered information but reached the maximum number of iterations. Here's what I found."
        );

        // 10 iterations for the first attempt, 5 for the recovery attempt
        assert_eq!(llm.call_count(), 15);

        // Only the recovery attempt's tool results are reported back
        assert_eq!(
            body["tool_calls"]
                .as_array()
                .expect("tool results should be reported")
                .len(),
            5
        );
        assert_eq!(body["tool_calls"][0]["tool_name"], MISSING_TOOL);

        // The user turn survives, the first attempt's 20 messages were rolled back,
        // and the recovery attempt's 5 x (tool call + tool result) remain.
        let stored = memory
            .get_messages(body["conversation_id"].as_str().expect("id"))
            .await
            .expect("Failed to read the conversation back");
        assert_eq!(stored.len(), 11);
        assert_eq!(stored[0].role, MessageRole::User);
        assert_eq!(stored[0].content.text(), "go in circles");

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_reports_a_failing_recovery_attempt_as_a_server_error() {
        // Ten tool-call replies drive the first attempt into its iteration cap; the
        // recovery attempt then gets a body it cannot parse.
        let mut bodies: Vec<String> = (0..10)
            .map(|i| tool_call_completion(&format!("call_{}", i), MISSING_TOOL, "{}"))
            .collect();
        bodies.push("<html>gateway error</html>".to_string());
        let llm = MockLlm::start(MockLlmConfig::replying_with_bodies(bodies)).await;
        let (_dir, memory) = memory_with(vec![]).await;

        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({ "message": "go in circles" }),
        )
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        let body = String::from_utf8_lossy(&body).to_string();
        assert!(body.contains("Recovery failed"), "{}", body);
        assert_eq!(llm.call_count(), 11);

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_trims_an_oversized_conversation_after_answering() {
        let llm = MockLlm::start(MockLlmConfig::replying("ok")).await;

        // Message timestamps are whole seconds and the trim works on them, so the
        // old half of the history is written a full second before the recent half.
        let old: Vec<(&str, ChatMessage)> = (0..60)
            .map(|i| ("conv-1", message(MessageRole::User, &format!("old {}", i))))
            .collect();
        let (_dir, memory) = memory_with(old).await;
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        for i in 0..40 {
            memory
                .add_message(
                    "conv-1",
                    message(MessageRole::User, &format!("recent {}", i)),
                )
                .await
                .expect("Failed to store message");
        }

        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({ "message": "one more", "conversation_id": "conv-1" }),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(as_json(&body)["message"], "ok");

        // 100 + the new turn + the answer is over the 100 message cap, so the old
        // history is dropped. Note the store trims by timestamp rather than by count,
        // so every message sharing the cut-off second survives and more than the
        // requested 20 can remain.
        let stored = memory
            .get_messages("conv-1")
            .await
            .expect("Failed to read the conversation back");
        let texts: Vec<String> = stored.iter().map(|m| m.content.text()).collect();
        assert!(
            (20..=42).contains(&texts.len()),
            "expected the history to be trimmed, got {} messages",
            texts.len()
        );
        assert!(
            !texts.iter().any(|t| t.starts_with("old ")),
            "the older half should have been dropped: {:?}",
            texts
        );
        assert!(texts.contains(&"one more".to_string()), "{:?}", texts);
        assert!(texts.contains(&"ok".to_string()), "{:?}", texts);

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_reports_a_failing_llm_as_a_server_error() {
        let mut config = MockLlmConfig::replying("never read");
        config.chat_status = 500;
        let llm = MockLlm::start(config).await;
        let (_dir, memory) = memory_with(vec![]).await;

        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({ "message": "hi" }),
        )
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        let body = String::from_utf8_lossy(&body).to_string();
        assert!(body.contains("Agent loop failed"), "{}", body);
        assert!(body.contains("LLM server error"), "{}", body);

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_chat_reports_an_unreachable_llm_as_a_server_error() {
        let (_dir, memory) = memory_with(vec![]).await;

        // Port 1 is privileged and never bound, so the connection is refused
        let (status, body) = post_chat(
            "127.0.0.1",
            1,
            &memory,
            vec![],
            serde_json::json!({ "message": "hi" }),
        )
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            String::from_utf8_lossy(&body).contains("Agent loop failed"),
            "{}",
            String::from_utf8_lossy(&body)
        );

        // The user turn is still recorded, even though the answer never arrived
        let conversations = memory
            .get_conversations()
            .await
            .expect("Failed to list conversations");
        assert_eq!(conversations.len(), 1);
    }

    #[actix_web::test]
    async fn test_chat_reports_a_broken_store_as_a_server_error() {
        let llm = MockLlm::start(MockLlmConfig::replying("unused")).await;
        let (_dir, memory) = memory_with(vec![]).await;
        memory.drop_tables_for_tests().await;

        let (status, body) = post_chat(
            "127.0.0.1",
            llm.port(),
            &memory,
            vec![],
            serde_json::json!({ "message": "hi" }),
        )
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            String::from_utf8_lossy(&body).contains("Failed to get conversation ID"),
            "{}",
            String::from_utf8_lossy(&body)
        );
        assert_eq!(llm.call_count(), 0, "the model must not be consulted");

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_cancelling_a_live_generation_signals_the_watcher() {
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        let mut map = HashMap::new();
        map.insert("conv-1".to_string(), tx);
        let active: ActiveGenerations = Arc::new(Mutex::new(map));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::clone(&active)))
                .service(cancel_agent_generation),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/agent/chat/conv-1/cancel")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "cancelled");
        assert!(rx.has_changed().expect("the sender should still be alive"));
        assert!(*rx.borrow_and_update(), "cancellation should be signalled");
    }

    #[actix_web::test]
    async fn test_cancelling_an_unknown_generation_is_not_found() {
        let active: ActiveGenerations = Arc::new(Mutex::new(HashMap::new()));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(Arc::clone(&active)))
                .service(cancel_agent_generation),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/agent/chat/conv-1/cancel")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "No active generation found");
    }
}
