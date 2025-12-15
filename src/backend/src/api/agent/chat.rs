use crate::api::agent::agent_loop::{execute_agent_loop, AgentLoopConfig};
use crate::api::agent::sqlite_memory::SqliteConversationMemory;
use crate::api::agent::tools::{
    chromadb::ChromaDBTool, financial_data::FinancialDataTool, registry::ToolRegistry,
    selector::ToolSelector,
};
use crate::api::agent::types::{
    AgentChatRequest, AgentChatResponse, AgentConfig, ChatMessage, MessageRole, ToolType,
};
use crate::api::llama_server::types::Config;
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use reqwest::Client;
use std::sync::{Arc, Mutex};

/// Clean response text by removing internal reasoning markers and redacted content
fn clean_response(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Remove redacted reasoning markers
    cleaned = cleaned.replace("<|redacted_reasoning|>", "");
    cleaned = cleaned.replace("</think>", "");
    cleaned = cleaned.replace("<think>", "");
    cleaned = cleaned.replace("</think>", "");

    // Remove tool call markers
    cleaned = cleaned.replace("<｜tool▁calls▁begin｜>", "");
    cleaned = cleaned.replace("<｜tool▁calls▁end｜>", "");
    cleaned = cleaned.replace("<｜tool▁call▁begin｜>", "");
    cleaned = cleaned.replace("<｜tool▁call▁end｜>", "");
    cleaned = cleaned.replace("<｜tool▁sep｜>", "");
    cleaned = cleaned.replace("<｜tool▁outputs▁begin｜>", "");
    cleaned = cleaned.replace("<｜tool▁outputs▁end｜>", "");
    cleaned = cleaned.replace("<｜tool▁output▁begin｜>", "");
    cleaned = cleaned.replace("<｜tool▁output▁end｜>", "");

    // Remove common internal reasoning patterns (Thought/Action/Observation format)
    if cleaned.contains("Thought:")
        || cleaned.contains("Action:")
        || cleaned.contains("Observation:")
    {
        // Try to extract just the answer if present
        if let Some(answer_start) = cleaned.rfind("Answer:") {
            cleaned = cleaned[answer_start + 7..].trim().to_string();
        } else if let Some(answer_start) = cleaned.rfind("answer:") {
            cleaned = cleaned[answer_start + 7..].trim().to_string();
        } else {
            // If no Answer found, try to remove the reasoning blocks
            // Look for patterns like "Thought: ... Action: ... Observation: ..."
            let lines: Vec<&str> = cleaned.lines().collect();
            let mut filtered_lines = Vec::new();
            let mut skip_until_answer = false;

            for line in lines {
                let line_lower = line.trim().to_lowercase();
                if line_lower.starts_with("thought:")
                    || line_lower.starts_with("action:")
                    || line_lower.starts_with("observation:")
                    || line_lower.starts_with("current task:")
                    || line_lower.starts_with("you are in a new chain")
                {
                    skip_until_answer = true;
                    continue;
                }
                if line_lower.starts_with("answer:") {
                    skip_until_answer = false;
                    filtered_lines.push(&line[7..]); // Skip "Answer:" prefix
                    continue;
                }
                if !skip_until_answer {
                    filtered_lines.push(line);
                }
            }
            if !filtered_lines.is_empty() {
                cleaned = filtered_lines.join("\n");
            }
        }
    }

    // Remove any remaining HTML-like tags that might be internal markers
    // Use simple string replacement instead of regex for reliability
    let mut result = String::new();
    let mut in_tag = false;
    for ch in cleaned.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' && in_tag {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    cleaned = result;

    cleaned.trim().to_string()
}

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

    // Get or create conversation ID from SQLite
    let conversation_id = sqlite_memory
        .get_or_create_conversation_id(req.conversation_id.clone())
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
    if let Some(chromadb_tool_config) = &config.chromadb {
        match ChromaDBTool::new(chroma_address.as_str(), chromadb_tool_config.clone()) {
            Ok(tool) => {
                if let Err(e) = tool_registry.register(Arc::new(tool)) {
                    println!("⚠️ Failed to register ChromaDB tool: {}", e);
                }
            }
            Err(e) => {
                println!("⚠️ Failed to create ChromaDB tool: {}", e);
            }
        }
    }

    // Register Financial Data tool if enabled
    if config.enabled_tools.contains(&ToolType::FinancialData) {
        let financial_tool = FinancialDataTool::new();
        if let Err(e) = tool_registry.register(Arc::new(financial_tool)) {
            println!("⚠️ Failed to register Financial Data tool: {}", e);
        }
    }

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

    // Get tools by category for logging
    use crate::api::agent::tools::agent_tool::ToolCategory;
    let search_tools = tool_registry_arc.get_tools_by_category(ToolCategory::Search);
    let data_tools = tool_registry_arc.get_tools_by_category(ToolCategory::DataQuery);
    if !search_tools.is_empty() || !data_tools.is_empty() {
        println!(
            "📊 Tools by category: {} search, {} data query",
            search_tools.len(),
            data_tools.len()
        );
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

    // Check if this query requires tools (skip tool setup for simple greetings)
    let requires_tools = tool_selector.requires_tools(&req.message);

    // Build system prompt using tool selector
    let system_prompt = tool_selector.build_system_prompt();

    // Select most relevant tools for this query (for logging/debugging)
    if requires_tools && !tools.is_empty() {
        let selected_tools = tool_selector.select_tools(&req.message, Some(3));
        println!(
            "🎯 Selected {} relevant tool(s) for query",
            selected_tools.len()
        );
    }
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
        content: system_prompt,
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

    // Get model name from llama_server config
    let model_name = {
        let llama_config_guard = llama_config.lock().unwrap();
        llama_config_guard.hf_model.clone()
    };

    // Call llama.cpp server
    let llama_url = "http://localhost:8080/v1/chat/completions";
    let client = Client::new();

    println!(
        "🤖 Starting agent loop (conversation: {})...",
        conversation_id
    );

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

    // Execute agent loop - allows iterative tool use
    let loop_config = AgentLoopConfig::default();
    let mut loop_result = execute_agent_loop(
        &client,
        llama_url,
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
        println!("❌ Agent loop error: {}", e);
        actix_web::error::ErrorInternalServerError(format!("Agent loop failed: {}", e))
    })?;

    // If agent got stuck, recover by restarting with clean context
    if loop_result.stuck {
        println!("🔄 Agent got stuck, recovering with clean context...");

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
            content: system_prompt_clone,
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
            llama_url,
            model_name,
            recovery_messages,
            tools,
            tool_registry_arc,
            Arc::clone(&sqlite_memory),
            conversation_id.clone(),
            recovery_config,
        )
        .await
        .map_err(|e| {
            println!("❌ Recovery attempt failed: {}", e);
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

    // If conversation has more than 100 messages, clear it to prevent bloat
    if msg_count > 100 {
        println!(
            "🧹 Conversation {} has {} messages, clearing to prevent bloat",
            conversation_id, msg_count
        );
        if let Err(e) = sqlite_memory.clear_conversation(&conversation_id).await {
            println!("⚠️ Failed to clear conversation: {}", e);
        } else {
            println!("✅ Cleared conversation {}", conversation_id);
        }
    }

    println!(
        "✅ Agent loop completed after {} iterations",
        loop_result.iterations
    );

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
