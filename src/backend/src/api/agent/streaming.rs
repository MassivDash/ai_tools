use crate::api::agent::sqlite_memory::SqliteConversationMemory;
use crate::api::agent::tools::registry::ToolRegistry;
use crate::api::agent::types::{
    AgentStreamEvent, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, MessageRole,
    ToolCallResult,
};
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc;

use super::agent_loop::AgentLoopConfig;

/// Execute agent loop with streaming support
/// Sends events through the provided channel
#[allow(clippy::too_many_arguments)]
pub async fn execute_agent_loop_streaming(
    client: &Client,
    llama_url: &str,
    model_name: String,
    mut messages: Vec<ChatMessage>,
    tools: Vec<crate::api::agent::types::Tool>,
    tool_registry: Arc<ToolRegistry>,
    sqlite_memory: Arc<SqliteConversationMemory>,
    conversation_id: String,
    config: AgentLoopConfig,
    tx: mpsc::UnboundedSender<Result<AgentStreamEvent, anyhow::Error>>,
) -> Result<()> {
    let mut tool_results = Vec::new();
    let mut iterations = 0;

    loop {
        iterations += 1;

        // Send thinking status at start of each iteration
        if iterations == 1 {
            let _ = tx.send(Ok(AgentStreamEvent::Status {
                status: "thinking".to_string(),
                message: Some("Thinking...".to_string()),
            }));
        } else {
            let _ = tx.send(Ok(AgentStreamEvent::Status {
                status: "thinking".to_string(),
                message: Some(format!(
                    "Processing (iteration {}/{})...",
                    iterations, config.max_iterations
                )),
            }));
        }

        if iterations > config.max_iterations {
            let _ = tx.send(Ok(AgentStreamEvent::Status {
                status: "error".to_string(),
                message: Some("Maximum iterations reached".to_string()),
            }));

            let _final_message =
                "I've gathered information but reached the maximum number of iterations."
                    .to_string();

            let _ = tx.send(Ok(AgentStreamEvent::Done {
                conversation_id: Some(conversation_id.clone()),
                tool_calls: if tool_results.is_empty() {
                    None
                } else {
                    Some(tool_results)
                },
            }));
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
                tool_results_buffer.push(format!("{}: {}", tool_name, msg.content));
            } else {
                // If we have buffered tool results, create a user message with them
                if !tool_results_buffer.is_empty() {
                    let tool_results_content = tool_results_buffer.join("\n");
                    filtered_messages.push(ChatMessage {
                        role: MessageRole::User,
                        content: format!("Tool results:\n{}", tool_results_content),
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
                content: format!("Tool results:\n{}", tool_results_content),
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
        };

        let response = match client.post(llama_url).json(&request).send().await {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(Ok(AgentStreamEvent::Error {
                    message: format!("Request failed: {}", e),
                }));
                break;
            }
        };

        let response_status = response.status();
        let response_text = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                let _ = tx.send(Ok(AgentStreamEvent::Error {
                    message: format!("Failed to read response: {}", e),
                }));
                break;
            }
        };

        if !response_status.is_success() {
            let _ = tx.send(Ok(AgentStreamEvent::Error {
                message: format!(
                    "LLM server error (status {}): {}",
                    response_status, response_text
                ),
            }));
            break;
        }

        let completion_response: ChatCompletionResponse = match serde_json::from_str(&response_text)
        {
            Ok(r) => r,
            Err(e) => {
                let _ = tx.send(Ok(AgentStreamEvent::Error {
                    message: format!("Failed to parse response: {}", e),
                }));
                break;
            }
        };

        if completion_response.choices.is_empty() {
            let _ = tx.send(Ok(AgentStreamEvent::Error {
                message: "No choices in LLM response".to_string(),
            }));
            break;
        }

        let choice = completion_response.choices.first().unwrap();

        // Check if LLM wants to use tools
        if let Some(tool_calls) = &choice.message.tool_calls {
            // Send tool call events
            for tool_call in tool_calls {
                let tool_name = tool_call.function.name.clone();

                // Send tool call event
                let _ = tx.send(Ok(AgentStreamEvent::ToolCall {
                    tool_name: tool_name.clone(),
                    arguments: tool_call.function.arguments.clone(),
                }));

                // Send status update
                let _ = tx.send(Ok(AgentStreamEvent::Status {
                    status: "calling_tool".to_string(),
                    message: Some(format!("🔧 Calling {}...", tool_name)),
                }));

                // Send status that tool is executing
                let _ = tx.send(Ok(AgentStreamEvent::Status {
                    status: "tool_executing".to_string(),
                    message: Some(format!("⚙️ Executing {}...", tool_name)),
                }));

                // Execute tool (status updates are sent before and after)
                let tool_exec_start = std::time::Instant::now();
                match tool_registry.execute_tool_call(tool_call).await {
                    Ok(result) => {
                        let duration = tool_exec_start.elapsed();
                        // Send tool result first
                        let _ = tx.send(Ok(AgentStreamEvent::ToolResult {
                            tool_name: tool_call.function.name.clone(),
                            success: true,
                            result: Some(result.result.clone()),
                        }));
                        // Then send completion status
                        let _ = tx.send(Ok(AgentStreamEvent::Status {
                            status: "tool_complete".to_string(),
                            message: Some(format!(
                                "✅ {} completed ({:.1}s)",
                                tool_name,
                                duration.as_secs_f64()
                            )),
                        }));
                        tool_results.push(result.clone());
                    }
                    Err(e) => {
                        let duration = tool_exec_start.elapsed();
                        // Send tool result (error) first
                        let _ = tx.send(Ok(AgentStreamEvent::ToolResult {
                            tool_name: tool_call.function.name.clone(),
                            success: false,
                            result: Some(format!("Error: {}", e)),
                        }));
                        // Then send error status
                        let _ = tx.send(Ok(AgentStreamEvent::Status {
                            status: "tool_error".to_string(),
                            message: Some(format!(
                                "❌ {} failed after {:.1}s",
                                tool_name,
                                duration.as_secs_f64()
                            )),
                        }));
                        let error_result = ToolCallResult {
                            tool_name: tool_call.function.name.clone(),
                            result: format!("Error: {}", e),
                        };
                        tool_results.push(error_result);
                    }
                }
            }

            // Add assistant message with tool calls
            let assistant_message = choice.message.clone();
            messages.push(assistant_message);

            // Add tool results as tool messages
            for tool_call in tool_calls {
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
                    content: result.result.clone(),
                    name: Some(tool_call.function.name.clone()),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                    reasoning_content: None,
                };
                messages.push(tool_message);
            }

            // Continue loop
            continue;
        } else {
            // No tool calls - LLM has final answer
            let final_message = if choice.message.content.is_empty() {
                if !tool_results.is_empty() {
                    "I've gathered the requested information.".to_string()
                } else {
                    "I've processed your request.".to_string()
                }
            } else {
                choice.message.content.clone()
            };

            // Send status that we're finalizing
            let _ = tx.send(Ok(AgentStreamEvent::Status {
                status: "finalizing".to_string(),
                message: Some("Finalizing response...".to_string()),
            }));

            // Stream the final message character by character for typing effect
            // Send first chunk immediately to start streaming
            if !final_message.is_empty() {
                let first_chunk: String = final_message.chars().take(1).collect();
                if !first_chunk.is_empty() {
                    let _ = tx.send(Ok(AgentStreamEvent::TextChunk { text: first_chunk }));
                }
                // Stream the rest in chunks
                let remaining: String = final_message.chars().skip(1).collect();
                for chunk in remaining.chars().collect::<Vec<_>>().chunks(3) {
                    let chunk_text: String = chunk.iter().collect();
                    if !chunk_text.is_empty() {
                        let _ = tx.send(Ok(AgentStreamEvent::TextChunk { text: chunk_text }));
                        // Small delay for typing effect (increased for better visibility)
                        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
                    }
                }
            }

            // Store final assistant response
            let final_assistant_message = ChatMessage {
                role: MessageRole::Assistant,
                content: final_message.clone(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            };
            if let Err(e) = sqlite_memory
                .add_message(&conversation_id, final_assistant_message)
                .await
            {
                let _ = tx.send(Ok(AgentStreamEvent::Error {
                    message: format!("Failed to store message: {}", e),
                }));
            }

            // Send done event
            let _ = tx.send(Ok(AgentStreamEvent::Done {
                conversation_id: Some(conversation_id),
                tool_calls: if tool_results.is_empty() {
                    None
                } else {
                    Some(tool_results)
                },
            }));
            break;
        }
    }

    Ok(())
}
