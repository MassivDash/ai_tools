use crate::api::agent::tools::{chromadb::ChromaDBTool, financial_data::FinancialDataTool};
use crate::api::agent::types::{
    AgentChatRequest, AgentChatResponse, AgentConfig, ChatCompletionRequest,
    ChatCompletionResponse, ChatMessage, MessageRole, Tool, ToolCallResult, ToolType,
};
use crate::api::llama_server::types::Config;
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use anyhow::Context;
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
) -> ActixResult<HttpResponse> {
    let config = agent_config.lock().unwrap().clone();

    // Build tools for OpenAI-compatible API
    let mut tools = Vec::new();

    // Add ChromaDB tool if configured (ChromaDB is always available if configured, not a toggle)
    if let Some(_chromadb_config) = &config.chromadb {
        tools.push(Tool {
            tool_type: "function".to_string(),
            function: serde_json::from_value(ChromaDBTool::get_function_definition())
                .context("Failed to parse ChromaDB function definition")
                .map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!(
                        "Failed to parse function definition: {}",
                        e
                    ))
                })?,
        });
    }

    // Add Financial Data tool if enabled
    if config.enabled_tools.contains(&ToolType::FinancialData) {
        tools.push(Tool {
            tool_type: "function".to_string(),
            function: serde_json::from_value(FinancialDataTool::get_function_definition())
                .context("Failed to parse FinancialData function definition")
                .map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!(
                        "Failed to parse function definition: {}",
                        e
                    ))
                })?,
        });
    }

    // Build messages
    let mut messages = vec![ChatMessage {
        role: MessageRole::System,
        content: "You are a helpful AI assistant with access to tools. 

IMPORTANT GUIDELINES FOR TOOL USAGE:
- DO NOT use tools for casual greetings, small talk, or general conversation (e.g., 'hello', 'how are you', 'how u doin', 'thanks', etc.)
- ALWAYS use search_chromadb when the user asks about technical topics, programming frameworks/libraries (like Bevy, React, Rust, etc.), code examples, documentation, or specific implementations - even if you have general knowledge, the knowledge base may have detailed, specific, or up-to-date information
- Use search_chromadb for questions about specific people, places, events, technical details, or documents that might be in the knowledge base
- When you receive search results, USE THEM COMPREHENSIVELY: read through ALL the results, synthesize information from multiple documents, combine details from different sources, and provide detailed, thorough answers based on what you found
- If search results contain relevant information, provide a comprehensive answer that incorporates details from ALL relevant results - don't just give a brief summary. Include specific examples, code snippets, explanations, and details from the documents
- For technical topics like frameworks, libraries, or code examples, synthesize information from multiple documents to give a complete picture
- For technical topics, use 8-10 search results to get comprehensive information
- If search results are not relevant or don't contain useful information, simply inform the user that you couldn't find relevant information - do not mention similarity scores or technical details about the search process
- Always respond naturally and conversationally - provide information directly without explaining how you found it
- Do not include internal reasoning, tool call formats, similarity scores, or technical search details in your responses
- When in doubt about technical topics, frameworks, or libraries, SEARCH - the knowledge base likely has specific information".to_string(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    }];

    // Add user message
    messages.push(ChatMessage {
        role: MessageRole::User,
        content: req.message.clone(),
        name: None,
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    });

    // Get model name from llama_server config
    let model_name = {
        let llama_config_guard = llama_config.lock().unwrap();
        // Use the hf_model as the model identifier, or fallback to "llama"
        llama_config_guard.hf_model.clone()
    };

    // Call llama.cpp server
    let llama_url = "http://localhost:8080/v1/chat/completions";
    let client = Client::new();

    let completion_request = ChatCompletionRequest {
        messages: messages.clone(),
        model: model_name,
        temperature: Some(0.7),
        max_tokens: Some(2000),
        tools: if tools.is_empty() {
            None
        } else {
            Some(tools.clone())
        },
        tool_choice: if tools.is_empty() {
            None
        } else {
            Some("auto".to_string())
        },
    };

    println!("🤖 Sending chat request to llama.cpp server...");
    let response = client
        .post(llama_url)
        .json(&completion_request)
        .send()
        .await
        .map_err(|e| {
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to connect to llama.cpp server: {}",
                e
            ))
        })?;

    let response_status = response.status();
    if !response_status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        println!(
            "❌ llama.cpp server error (status {}): {}",
            response_status, error_text
        );
        return Ok(HttpResponse::BadGateway().json(AgentChatResponse {
            success: false,
            message: format!("llama.cpp server error: {}", error_text),
            conversation_id: req.conversation_id.clone(),
            tool_calls: None,
        }));
    }

    // Get response text first to debug
    let response_text = response.text().await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Failed to read response body: {}", e))
    })?;

    println!("📥 llama.cpp response: {}", response_text);

    // Try to parse the response
    let completion_response: ChatCompletionResponse = serde_json::from_str(&response_text)
        .map_err(|e| {
            println!("❌ Failed to parse response JSON: {}", e);
            println!("📄 Response body: {}", response_text);
            actix_web::error::ErrorInternalServerError(format!(
                "Failed to parse llama.cpp response: {}. Response: {}",
                e, response_text
            ))
        })?;

    // Handle tool calls if present
    let mut tool_results = Vec::new();

    if completion_response.choices.is_empty() {
        println!("⚠️ No choices in llama.cpp response");
        return Ok(HttpResponse::BadGateway().json(AgentChatResponse {
            success: false,
            message: "No response choices from llama.cpp server".to_string(),
            conversation_id: req.conversation_id.clone(),
            tool_calls: None,
        }));
    }

    let final_message = if let Some(choice) = completion_response.choices.first() {
        if let Some(tool_calls) = &choice.message.tool_calls {
            // Execute tool calls
            println!("🔧 Executing {} tool call(s)...", tool_calls.len());

            for tool_call in tool_calls {
                match tool_call.function.name.as_str() {
                    "search_chromadb" => {
                        if let Some(chromadb_tool_config) = &config.chromadb {
                            match ChromaDBTool::new(
                                chroma_address.as_str(),
                                chromadb_tool_config.clone(),
                            ) {
                                Ok(tool) => match tool.execute_tool_call(tool_call).await {
                                    Ok(result) => {
                                        tool_results.push(result.clone());
                                        println!("✅ ChromaDB search completed");
                                    }
                                    Err(e) => {
                                        println!("❌ Tool execution error: {}", e);
                                        tool_results.push(ToolCallResult {
                                            tool_name: "search_chromadb".to_string(),
                                            result: format!("Error: {}", e),
                                        });
                                    }
                                },
                                Err(e) => {
                                    println!("❌ Failed to create ChromaDB tool: {}", e);
                                    tool_results.push(ToolCallResult {
                                        tool_name: "search_chromadb".to_string(),
                                        result: format!("Error: Failed to initialize tool: {}", e),
                                    });
                                }
                            }
                        }
                    }
                    "get_financial_data" => {
                        match FinancialDataTool::execute_tool_call(tool_call).await {
                            Ok(result) => {
                                tool_results.push(result.clone());
                                println!("✅ Financial data retrieved");
                            }
                            Err(e) => {
                                println!("❌ Financial data tool error: {}", e);
                                tool_results.push(ToolCallResult {
                                    tool_name: "get_financial_data".to_string(),
                                    result: format!("Error: {}", e),
                                });
                            }
                        }
                    }
                    _ => {
                        println!("⚠️ Unknown tool: {}", tool_call.function.name);
                    }
                }
            }

            // If we have tool results, make a follow-up call with the results
            if !tool_results.is_empty() {
                // Add assistant message with tool calls
                messages.push(choice.message.clone());

                // Add tool results as tool messages
                for (i, tool_call) in tool_calls.iter().enumerate() {
                    if let Some(result) = tool_results.get(i) {
                        messages.push(ChatMessage {
                            role: MessageRole::Tool,
                            content: result.result.clone(),
                            name: Some(tool_call.function.name.clone()),
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                            reasoning_content: None,
                        });
                    }
                }

                // Get model name again for follow-up request
                let model_name_followup = {
                    let llama_config_guard = llama_config.lock().unwrap();
                    llama_config_guard.hf_model.clone()
                };

                // Make a follow-up call to get the final response
                let follow_up_request = ChatCompletionRequest {
                    messages: messages.clone(),
                    model: model_name_followup,
                    temperature: Some(0.7),
                    max_tokens: Some(2000),
                    tools: if tools.is_empty() { None } else { Some(tools) },
                    tool_choice: Some("none".to_string()), // Don't allow more tool calls
                };

                println!("🤖 Getting final response from llama.cpp server...");
                let follow_up_response = client
                    .post(llama_url)
                    .json(&follow_up_request)
                    .send()
                    .await
                    .map_err(|e| {
                        actix_web::error::ErrorInternalServerError(format!(
                            "Failed to connect to llama.cpp server: {}",
                            e
                        ))
                    })?;

                let follow_up_status = follow_up_response.status();
                // Get response text first to debug (can only read once)
                let follow_up_text = follow_up_response.text().await.map_err(|e| {
                    actix_web::error::ErrorInternalServerError(format!(
                        "Failed to read follow-up response body: {}",
                        e
                    ))
                })?;

                println!(
                    "📥 llama.cpp follow-up response (status {}): {}",
                    follow_up_status, follow_up_text
                );

                if follow_up_status.is_success() {
                    let follow_up_completion: ChatCompletionResponse =
                        serde_json::from_str(&follow_up_text).map_err(|e| {
                            println!("❌ Failed to parse follow-up response JSON: {}", e);
                            println!("📄 Follow-up response body: {}", follow_up_text);
                            actix_web::error::ErrorInternalServerError(format!(
                                "Failed to parse llama.cpp follow-up response: {}. Response: {}",
                                e, follow_up_text
                            ))
                        })?;

                    if let Some(choice) = follow_up_completion.choices.first() {
                        let mut msg = choice.message.content.clone();
                        if msg.is_empty() {
                            msg = "I processed your request but got an empty response.".to_string();
                        } else {
                            // Clean up any internal reasoning markers or redacted content
                            msg = clean_response(&msg);
                        }
                        msg
                    } else {
                        "No response from llama.cpp server".to_string()
                    }
                } else {
                    println!(
                        "❌ llama.cpp follow-up error (status {}): {}",
                        follow_up_status, follow_up_text
                    );
                    format!("Error getting final response: {}", follow_up_text)
                }
            } else {
                choice.message.content.clone()
            }
        } else {
            let mut msg = choice.message.content.clone();
            if msg.is_empty() {
                msg = "I received your message but got an empty response.".to_string();
            } else {
                // Clean up any internal reasoning markers or redacted content
                msg = clean_response(&msg);
            }
            msg
        }
    } else {
        return Ok(HttpResponse::BadGateway().json(AgentChatResponse {
            success: false,
            message: "No choices in llama.cpp response".to_string(),
            conversation_id: req.conversation_id.clone(),
            tool_calls: None,
        }));
    };

    Ok(HttpResponse::Ok().json(AgentChatResponse {
        success: true,
        message: final_message,
        conversation_id: req.conversation_id.clone(),
        tool_calls: if tool_results.is_empty() {
            None
        } else {
            Some(tool_results)
        },
    }))
}
