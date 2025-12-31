use crate::api::agent::core::agent_loop::{execute_agent_loop, AgentLoopConfig};
use crate::api::agent::core::streaming::execute_agent_loop_streaming;
use crate::api::agent::core::types::{
    AgentChatRequest, AgentChatResponse, AgentConfig, AgentStreamEvent, ChatMessage,
    MessageContent, MessageRole,
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
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use futures::StreamExt;
use reqwest::Client;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

/// Chat completion endpoint
#[post("/api/agent/chat")]
pub async fn agent_chat(
    req: web::Json<AgentChatRequest>,
    agent_config: web::Data<Arc<Mutex<AgentConfig>>>,
    chroma_address: web::Data<String>,
    _chromadb_config: web::Data<Arc<Mutex<crate::api::chromadb::config::types::ChromaDBConfig>>>,
    llama_config: web::Data<Arc<Mutex<Config>>>,
    sqlite_memory: web::Data<Arc<SqliteConversationMemory>>,
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
    // Register all enabled tools
    let context = tools::RegisterContext {
        chroma_address: Some(chroma_address.as_str()),
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
    let user_message = ChatMessage {
        role: MessageRole::User,
        content: req.message.clone(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    };
    messages_with_system.push(user_message.clone());

    // Store user message in SQLite
    sqlite_memory
        .add_message(&conversation_id, user_message)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to store user message: {}",
                e
            ))
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
    let loop_config = AgentLoopConfig::default();
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
pub async fn agent_chat_stream(
    req: web::Json<AgentChatRequest>,
    agent_config: web::Data<Arc<Mutex<AgentConfig>>>,
    chroma_address: web::Data<String>,
    _chromadb_config: web::Data<Arc<Mutex<crate::api::chromadb::config::types::ChromaDBConfig>>>,
    llama_config: web::Data<Arc<Mutex<Config>>>,
    sqlite_memory: web::Data<Arc<SqliteConversationMemory>>,
    agent_ws_state: web::Data<Arc<AgentWebSocketState>>,
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

    // Register all enabled tools
    let context = tools::RegisterContext {
        chroma_address: Some(chroma_address.as_str()),
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

    let user_message = ChatMessage {
        role: MessageRole::User,
        content: req.message.clone(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    };
    messages_with_system.push(user_message.clone());

    // Store user message
    sqlite_memory
        .add_message(&conversation_id, user_message)
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to store user message: {}",
                e
            ))
        })?;

    // model_name is already retrieved above

    let llama_url = format!("{}/v1/chat/completions", llama_base_url);
    let client = Client::new();

    // Create channel for streaming events (SSE)
    let (tx, rx) = mpsc::unbounded_channel::<Result<AgentStreamEvent, anyhow::Error>>();

    // Clone necessary data for the streaming task
    let client_clone = client.clone();
    let llama_url_clone = llama_url.to_string();
    let model_name_clone = model_name.clone();
    let tools_clone = tools.clone();
    let tool_registry_clone = Arc::clone(&tool_registry_arc);
    let sqlite_memory_clone = sqlite_memory.get_ref().clone();
    let conversation_id_clone = conversation_id.clone();
    let agent_ws_state_clone = agent_ws_state.get_ref().clone();
    let loop_config = AgentLoopConfig::default();

    // Spawn the agent loop in a background task
    // Events will be sent to both SSE (via tx) and WebSocket (via agent_ws_state)
    actix_rt::spawn(async move {
        // Create a wrapper sender that broadcasts to both SSE and WebSocket
        let tx_sse = tx.clone();
        let agent_ws_broadcast = agent_ws_state_clone.clone();
        let (tx_wrapper, mut rx_wrapper) =
            mpsc::unbounded_channel::<Result<AgentStreamEvent, anyhow::Error>>();

        // Spawn task to duplicate events to both SSE and WebSocket
        actix_rt::spawn(async move {
            while let Some(event_result) = rx_wrapper.recv().await {
                // Broadcast to WebSocket first (if successful)
                if let Ok(event) = &event_result {
                    agent_ws_broadcast.broadcast(event);
                }
                // Send to SSE (need to handle error case)
                match &event_result {
                    Ok(event) => {
                        let _ = tx_sse.send(Ok(event.clone()));
                    }
                    Err(e) => {
                        let _ = tx_sse.send(Err(anyhow::anyhow!("{}", e)));
                    }
                }
            }
        });

        if let Err(e) = execute_agent_loop_streaming(
            &client_clone,
            &llama_url_clone,
            model_name_clone.clone(), // Clone for the loop
            messages_with_system,
            tools_clone,
            tool_registry_clone,
            sqlite_memory_clone.clone(),   // Clone for the loop
            conversation_id_clone.clone(), // Clone for the loop
            loop_config,
            tx_wrapper,
        )
        .await
        {
            println!("Streaming agent loop error: {}", e);
        }

        // Attempt naming after stream finishes
        attempt_conversation_naming(
            client_clone,
            llama_url_clone,
            model_name_clone,
            sqlite_memory_clone, // already Arc in this scope
            conversation_id_clone,
        )
        .await;
    });

    // Convert events to SSE format
    let stream = UnboundedReceiverStream::new(rx).map(
        move |event_result| -> Result<web::Bytes, actix_web::Error> {
            match event_result {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                    Ok(web::Bytes::from(format!("data: {}\n\n", json)))
                }
                Err(e) => {
                    let error_event = AgentStreamEvent::Error {
                        message: e.to_string(),
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
