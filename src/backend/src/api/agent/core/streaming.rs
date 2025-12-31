use crate::api::agent::core::types::{
    AgentStreamEvent, ChatCompletionRequest, ChatMessage, MessageContent, MessageRole,
    ToolCallResult,
};
use crate::api::agent::memory::sqlite_memory::SqliteConversationMemory;

use crate::api::agent::tools::framework::registry::ToolRegistry;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::agent_loop::AgentLoopConfig;
use super::utils::{format_tool_status_message, StatusType};

/// Execute agent loop with streaming support
/// Sends events through the provided channel
#[allow(clippy::too_many_arguments)]
pub async fn execute_agent_loop_streaming(
    client: &Client,
    llama_url: &str,
    model_name: String,
    mut messages: Vec<ChatMessage>,
    tools: Vec<crate::api::agent::core::types::Tool>,
    tool_registry: Arc<ToolRegistry>,
    sqlite_memory: Arc<SqliteConversationMemory>,
    conversation_id: String,
    config: AgentLoopConfig,
    tx: mpsc::Sender<Result<AgentStreamEvent, anyhow::Error>>,
    mut cancel_rx: tokio::sync::watch::Receiver<bool>,
) -> Result<()> {
    let mut tool_results = Vec::new();
    let mut iterations = 0;
    let mut total_usage: Option<crate::api::agent::core::types::Usage> = None;

    loop {
        iterations += 1;

        // Check for cancellation at start of iteration
        if *cancel_rx.borrow() {
            println!("⚠️ Cancellation signal received at start of iteration");
            break;
        }

        // Send thinking status at start of each iteration
        if iterations == 1 {
            if tx
                .send(Ok(AgentStreamEvent::Status {
                    status: "thinking".to_string(),
                    message: Some("Thinking...".to_string()),
                }))
                .await
                .is_err()
            {
                break;
            }
        } else if tx
            .send(Ok(AgentStreamEvent::Status {
                status: "thinking".to_string(),
                message: Some(format!(
                    "Processing (iteration {}/{})...",
                    iterations, config.max_iterations
                )),
            }))
            .await
            .is_err()
        {
            break;
        }

        if iterations > config.max_iterations {
            let _ = tx
                .send(Ok(AgentStreamEvent::Status {
                    status: "error".to_string(),
                    message: Some("Maximum iterations reached".to_string()),
                }))
                .await;

            let _final_message =
                "I've gathered information but reached the maximum number of iterations."
                    .to_string();

            let _ = tx
                .send(Ok(AgentStreamEvent::Done {
                    conversation_id: Some(conversation_id.clone()),
                    tool_calls: if tool_results.is_empty() {
                        None
                    } else {
                        Some(tool_results)
                    },
                    usage: total_usage,
                }))
                .await;
            break;
        }

        // Build request - convert tool messages to user messages to maintain alternation
        // (the LLM server expects alternating user/assistant and doesn't allow prefill with tool_calls)
        let mut filtered_messages: Vec<ChatMessage> = Vec::new();
        let mut tool_results_buffer: Vec<String> = Vec::new();

        for msg in messages.iter() {
            if matches!(msg.role, MessageRole::Tool) {
                // Collect tool results to create a user message
                let tool_name = msg.name.as_deref().unwrap_or("unknown");
                tool_results_buffer.push(format!("{}: {}", tool_name, msg.content.text()));
            } else {
                // If we have buffered tool results, create a user message with them
                if !tool_results_buffer.is_empty() {
                    let tool_results_content = tool_results_buffer.join("\n");
                    filtered_messages.push(ChatMessage {
                        role: MessageRole::User,
                        content: MessageContent::Text(format!(
                            "Tool results:\n{}",
                            tool_results_content
                        )),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                    tool_results_buffer.clear();
                }
                filtered_messages.push(msg.clone());
            }
        }

        // Handle any remaining tool results at the end
        if !tool_results_buffer.is_empty() {
            let tool_results_content = tool_results_buffer.join("\n");
            filtered_messages.push(ChatMessage {
                role: MessageRole::User,
                content: MessageContent::Text(format!("Tool results:\n{}", tool_results_content)),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }

        let tool_choice = if !tools.is_empty() {
            Some("auto".to_string())
        } else {
            None
        };

        // Enable streaming in request
        let request = ChatCompletionRequest {
            messages: filtered_messages,
            model: model_name.clone(),
            temperature: Some(config.temperature),
            max_tokens: Some(config.max_tokens),
            tools: if tools.is_empty() {
                None
            } else {
                Some(tools.clone())
            },
            tool_choice,
            stream: Some(true),
        };

        // Send request with cancellation check
        let mut response = tokio::select! {
            res = client.post(llama_url).json(&request).send() => {
                match res {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(Ok(AgentStreamEvent::Error {
                            message: format!("Request failed: {}", e),
                        })).await;
                        break;
                    }
                }
            }
            _ = cancel_rx.changed() => {
                println!("⚠️ Cancellation signal received during request setup");
                break;
            }
        };

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            let _ = tx
                .send(Ok(AgentStreamEvent::Error {
                    message: format!("LLM server error: {}", error_text),
                }))
                .await;
            break;
        }

        // Variables to accumulate streamed response
        let mut accumulated_content = String::new();
        let mut accumulated_tool_calls: Vec<crate::api::agent::core::types::ToolCall> = Vec::new();
        let mut final_usage: Option<crate::api::agent::core::types::Usage> = None;
        let mut loop_cancelled = false;

        // Process SSE stream
        loop {
            tokio::select! {
                chunk_option = response.chunk() => {
                    match chunk_option {
                        Ok(Some(chunk)) => {
                            let chunk_str = String::from_utf8_lossy(&chunk);
                            for line in chunk_str.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        continue;
                                    }

                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                        // Extract usage if present
                                        if let Some(usage_val) = json.get("usage") {
                                            if let Ok(usage) = serde_json::from_value(usage_val.clone()) {
                                                final_usage = Some(usage);
                                            }
                                        }

                                        // Extract choices
                                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                                            if let Some(choice) = choices.first() {
                                                // Process delta
                                                if let Some(delta) = choice.get("delta") {
                                                    // 1. Handle Content Streaming
                                                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                                        if !content.is_empty() {
                                                            accumulated_content.push_str(content);
                                                            // Stream text directly to client
                                                            if tx.send(Ok(AgentStreamEvent::TextChunk { text: content.to_string() })).await.is_err() {
                                                                // Client disconnected, treat as cancellation
                                                                loop_cancelled = true;
                                                            }
                                                        }
                                                    }

                                                    // 2. Handle Tool Calls Streaming
                                                    if let Some(tool_calls_arr) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                                        for tc in tool_calls_arr {
                                                            let index = tc.get("index").and_then(|i| i.as_u64()).map(|i| i as usize);

                                                            if let Some(idx) = index {
                                                                // Ensure vector is large enough
                                                                while accumulated_tool_calls.len() <= idx {
                                                                    accumulated_tool_calls.push(crate::api::agent::core::types::ToolCall {
                                                                        id: String::new(),
                                                                        tool_type: "function".to_string(),
                                                                        function: crate::api::agent::core::types::FunctionCall {
                                                                            name: String::new(),
                                                                            arguments: String::new(),
                                                                        },
                                                                    });
                                                                }

                                                                let current_tool = &mut accumulated_tool_calls[idx];

                                                                // Update ID if present
                                                                if let Some(id) = tc.get("id").and_then(|s| s.as_str()) {
                                                                    current_tool.id = id.to_string();
                                                                }

                                                                // Update function details
                                                                if let Some(function) = tc.get("function") {
                                                                    if let Some(name) = function.get("name").and_then(|s| s.as_str()) {
                                                                        current_tool.function.name = name.to_string();
                                                                    }
                                                                    if let Some(args) = function.get("arguments").and_then(|s| s.as_str()) {
                                                                        current_tool.function.arguments.push_str(args);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if loop_cancelled {
                                break;
                            }
                        }
                        Ok(None) => break, // Check streaming finished
                        Err(e) => {
                             let _ = tx.send(Ok(AgentStreamEvent::Error {
                                message: format!("Stream error: {}", e),
                            })).await;
                            loop_cancelled = true;
                            break;
                        }
                    }
                }
                _ = cancel_rx.changed() => {
                    println!("⚠️ Cancellation signal received during streaming");
                    loop_cancelled = true;
                    break;
                }
            }
        }

        // Handle Cancellation - SAVE STATE
        if loop_cancelled {
            if !accumulated_content.is_empty() {
                println!("💾 Saving partial response due to cancellation...");
                let partial_assistant_message = ChatMessage {
                    role: MessageRole::Assistant,
                    content: MessageContent::Text(accumulated_content),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                };
                if let Err(e) = sqlite_memory
                    .add_message(&conversation_id, partial_assistant_message)
                    .await
                {
                    println!("Failed to save partial message: {}", e);
                }
                let _ = tx
                    .send(Ok(AgentStreamEvent::Done {
                        conversation_id: Some(conversation_id),
                        tool_calls: None,
                        usage: total_usage,
                    }))
                    .await;
            }
            break;
        }

        // Accumulate final usage
        if let Some(usage) = final_usage {
            if let Some(ref mut total) = total_usage {
                total.prompt_tokens = usage.prompt_tokens; // Usually cumulative in last chunk
                total.completion_tokens = usage.completion_tokens;
                total.total_tokens = usage.total_tokens;
            } else {
                total_usage = Some(usage);
            }
        }

        // Decide next step: Tool Execution or Final Answer
        if !accumulated_tool_calls.is_empty() {
            // Send tool call events
            let tool_calls_to_process = accumulated_tool_calls.clone();

            for tool_call in &tool_calls_to_process {
                let tool_name = tool_call.function.name.clone();

                // Get tool metadata for better status messages
                let tool_metadata = tool_registry
                    .get_tool_by_name(&tool_name)
                    .map(|t| t.metadata().clone());
                let display_name = tool_metadata
                    .as_ref()
                    .map(|m| m.name.clone())
                    .unwrap_or_else(|| tool_name.clone());

                // Send tool call event
                let _ = tx
                    .send(Ok(AgentStreamEvent::ToolCall {
                        tool_name: tool_name.clone(),
                        display_name: Some(display_name.clone()),
                        arguments: tool_call.function.arguments.clone(),
                    }))
                    .await;

                // Check cancellation before tool execution
                if *cancel_rx.borrow() {
                    loop_cancelled = true;
                    break;
                }

                // Send status updates and execute...

                let status_msg = format_tool_status_message(
                    &display_name,
                    tool_metadata.as_ref(),
                    StatusType::Calling,
                );
                let _ = tx
                    .send(Ok(AgentStreamEvent::Status {
                        status: "calling_tool".to_string(),
                        message: Some(status_msg),
                    }))
                    .await;

                let status_msg = format_tool_status_message(
                    &display_name,
                    tool_metadata.as_ref(),
                    StatusType::Executing,
                );
                let _ = tx
                    .send(Ok(AgentStreamEvent::Status {
                        status: "tool_executing".to_string(),
                        message: Some(status_msg),
                    }))
                    .await;

                // Execute tool
                let tool_exec_start = std::time::Instant::now();
                match tool_registry.execute_tool_call(tool_call).await {
                    Ok(result) => {
                        let duration = tool_exec_start.elapsed();
                        // Send tool result first
                        let _ = tx
                            .send(Ok(AgentStreamEvent::ToolResult {
                                tool_name: tool_call.function.name.clone(),
                                display_name: Some(display_name.clone()),
                                success: true,
                                result: Some(result.result.clone()),
                            }))
                            .await;
                        // Then send completion status
                        let status_msg = format_tool_status_message(
                            &display_name,
                            tool_metadata.as_ref(),
                            StatusType::Complete(duration),
                        );
                        let _ = tx
                            .send(Ok(AgentStreamEvent::Status {
                                status: "tool_complete".to_string(),
                                message: Some(status_msg),
                            }))
                            .await;
                        tool_results.push(result.clone());
                    }
                    Err(e) => {
                        let duration = tool_exec_start.elapsed();
                        // Send tool result (error) first
                        let _ = tx
                            .send(Ok(AgentStreamEvent::ToolResult {
                                tool_name: tool_call.function.name.clone(),
                                display_name: Some(display_name.clone()),
                                success: false,
                                result: Some(format!("Error: {}", e)),
                            }))
                            .await;
                        // Then send error status
                        let status_msg = format_tool_status_message(
                            &display_name,
                            tool_metadata.as_ref(),
                            StatusType::Error(duration),
                        );
                        let _ = tx
                            .send(Ok(AgentStreamEvent::Status {
                                status: "tool_error".to_string(),
                                message: Some(status_msg),
                            }))
                            .await;
                        let error_result = ToolCallResult {
                            tool_name: tool_call.function.name.clone(),
                            result: format!("Error: {}", e),
                        };
                        tool_results.push(error_result);
                    }
                }
            }

            if loop_cancelled {
                // If cancelled during tool calls, we should probably save what we have?
                // Assistant message with tool calls was not yet added to DB in my previous logic (it was added after loop).
                // Let's add it now if we have it?
                // For now, simple break is safer, user can retry.
                break;
            }

            // Create and store assistant message with tool calls
            let assistant_message = ChatMessage {
                role: MessageRole::Assistant,
                content: if !accumulated_content.is_empty() {
                    MessageContent::Text(accumulated_content)
                } else {
                    MessageContent::Text(String::new())
                },
                name: None,
                tool_calls: Some(accumulated_tool_calls),
                tool_call_id: None,
                reasoning_content: None,
            };
            messages.push(assistant_message);

            // Add tool results as tool messages
            let tool_calls_to_msg_process = messages
                .last()
                .unwrap()
                .tool_calls
                .as_ref()
                .unwrap()
                .clone();
            for tool_call in tool_calls_to_msg_process {
                let result = tool_results
                    .iter()
                    .find(|r| r.tool_name == tool_call.function.name)
                    .cloned()
                    .unwrap_or_else(|| ToolCallResult {
                        tool_name: tool_call.function.name.clone(),
                        result: String::new(),
                    });

                let tool_message = ChatMessage {
                    role: MessageRole::Tool,
                    content: MessageContent::Text(result.result.clone()),
                    name: Some(tool_call.function.name.clone()),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                    reasoning_content: None,
                };
                messages.push(tool_message);
            }

            continue;
        } else {
            // Final Answer Handling
            let final_message = if accumulated_content.is_empty() {
                if !tool_results.is_empty() {
                    "I've gathered the requested information.".to_string()
                } else {
                    "I've processed your request.".to_string()
                }
            } else {
                accumulated_content.clone()
            };

            // Send status that we're finalizing
            if tx
                .send(Ok(AgentStreamEvent::Status {
                    status: "finalizing".to_string(),
                    message: Some("Finalizing response...".to_string()),
                }))
                .await
                .is_err()
            {
                return Ok(());
            }

            // Store final assistant response
            let final_assistant_message = ChatMessage {
                role: MessageRole::Assistant,
                content: MessageContent::Text(final_message.clone()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            };
            if let Err(e) = sqlite_memory
                .add_message(&conversation_id, final_assistant_message)
                .await
            {
                let _ = tx
                    .send(Ok(AgentStreamEvent::Error {
                        message: format!("Failed to store message: {}", e),
                    }))
                    .await;
            }

            let _ = tx
                .send(Ok(AgentStreamEvent::Done {
                    conversation_id: Some(conversation_id),
                    tool_calls: if tool_results.is_empty() {
                        None
                    } else {
                        Some(tool_results)
                    },
                    usage: total_usage,
                }))
                .await;
            break;
        }
    }

    Ok(())
}
