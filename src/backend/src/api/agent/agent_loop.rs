use crate::api::agent::sqlite_memory::SqliteConversationMemory;
use crate::api::agent::tools::registry::ToolRegistry;
use crate::api::agent::types::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, MessageRole, ToolCallResult,
};
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;

/// Agent loop result
pub struct AgentLoopResult {
    pub final_message: String,
    pub tool_calls: Vec<ToolCallResult>,
    pub iterations: usize,
    pub stuck: bool, // True if loop reached max iterations
}

/// Configuration for agent loop
pub struct AgentLoopConfig {
    pub max_iterations: usize,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10, // Maximum tool-call iterations
            max_tokens: 2000,
            temperature: 0.7,
        }
    }
}

/// Execute agent loop - allows LLM to use tools iteratively until it decides it has enough info
#[allow(clippy::too_many_arguments)]
pub async fn execute_agent_loop(
    client: &Client,
    llama_url: &str,
    model_name: String,
    mut messages: Vec<ChatMessage>,
    tools: Vec<crate::api::agent::types::Tool>,
    tool_registry: Arc<ToolRegistry>,
    sqlite_memory: Arc<SqliteConversationMemory>,
    conversation_id: String,
    config: AgentLoopConfig,
) -> Result<AgentLoopResult> {
    let mut tool_results = Vec::new();
    let mut iterations = 0;

    loop {
        iterations += 1;
        println!(
            "🔄 Agent loop iteration {}/{}",
            iterations, config.max_iterations
        );

        if iterations > config.max_iterations {
            println!("⚠️ Maximum iterations reached - agent appears stuck");
            // Get the last assistant message or create a default one
            let last_assistant = messages
                .iter()
                .rev()
                .find(|m| matches!(m.role, MessageRole::Assistant))
                .cloned();

            let final_message = if let Some(msg) = last_assistant {
                if !msg.content.is_empty() {
                    msg.content
                } else {
                    "I've gathered information but reached the maximum number of iterations. Here's what I found.".to_string()
                }
            } else {
                "I've processed your request but reached the maximum number of iterations."
                    .to_string()
            };

            return Ok(AgentLoopResult {
                final_message,
                tool_calls: tool_results,
                iterations,
                stuck: true,
            });
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

        println!("📤 Sending request to LLM (iteration {})...", iterations);
        let response = client.post(llama_url).json(&request).send().await?;

        let response_status = response.status();
        let response_text = response.text().await?;

        if !response_status.is_success() {
            return Err(anyhow::anyhow!(
                "LLM server error (status {}): {}",
                response_status,
                response_text
            ));
        }
        println!("📥 LLM response received (iteration {})", iterations);

        let completion_response: ChatCompletionResponse = serde_json::from_str(&response_text)
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse LLM response: {}. Response: {}",
                    e,
                    response_text
                )
            })?;

        if completion_response.choices.is_empty() {
            return Err(anyhow::anyhow!("No choices in LLM response"));
        }

        let choice = completion_response.choices.first().unwrap();

        // Check if LLM wants to use tools
        if let Some(tool_calls) = &choice.message.tool_calls {
            println!(
                "🔧 LLM requested {} tool call(s) in iteration {}",
                tool_calls.len(),
                iterations
            );

            // Execute all tool calls
            let mut iteration_tool_results = Vec::new();
            for tool_call in tool_calls {
                println!(
                    "   📞 Executing tool: {} with args: {}",
                    tool_call.function.name, tool_call.function.arguments
                );

                match tool_registry.execute_tool_call(tool_call).await {
                    Ok(result) => {
                        println!(
                            "   ✅ Tool '{}' executed successfully",
                            tool_call.function.name
                        );
                        iteration_tool_results.push((tool_call.clone(), result.clone()));
                        tool_results.push(result);
                    }
                    Err(e) => {
                        println!("   ❌ Tool execution error: {}", e);
                        let error_result = ToolCallResult {
                            tool_name: tool_call.function.name.clone(),
                            result: format!("Error: {}", e),
                        };
                        iteration_tool_results.push((tool_call.clone(), error_result.clone()));
                        tool_results.push(error_result);
                    }
                }
            }

            // Add assistant message with tool calls to conversation
            // Don't store tool call messages in SQLite - only final answers
            let assistant_message = choice.message.clone();
            messages.push(assistant_message);

            // Add tool results as tool messages
            // Don't store tool messages in SQLite - only user/assistant messages
            for (tool_call, result) in iteration_tool_results {
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

            // Continue loop - LLM will process tool results and decide next action
            println!("🔄 Continuing loop to process tool results...");
            continue;
        } else {
            // No tool calls - LLM has decided it has enough information
            let final_message = if choice.message.content.is_empty() {
                // If content is empty but we have tool results, synthesize from them
                if !tool_results.is_empty() {
                    "I've gathered the requested information.".to_string()
                } else {
                    "I've processed your request.".to_string()
                }
            } else {
                choice.message.content.clone()
            };

            println!(
                "✅ LLM provided final answer after {} iterations",
                iterations
            );

            // Store final assistant response in memory
            let final_assistant_message = ChatMessage {
                role: MessageRole::Assistant,
                content: final_message.clone(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            };
            sqlite_memory
                .add_message(&conversation_id, final_assistant_message)
                .await?;

            return Ok(AgentLoopResult {
                final_message,
                tool_calls: tool_results,
                iterations,
                stuck: false,
            });
        }
    }
}
