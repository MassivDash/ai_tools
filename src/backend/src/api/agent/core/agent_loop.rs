use crate::api::agent::core::logging::ConversationLogger;
use crate::api::agent::core::types::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, MessageContent, MessageRole,
    ToolCallResult,
};
use crate::api::agent::memory::sqlite_memory::SqliteConversationMemory;
use crate::api::agent::tools::framework::registry::ToolRegistry;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;

/// Agent loop result
#[derive(Debug)]
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
    pub debug_logging: bool,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10, // Maximum tool-call iterations
            max_tokens: 2000,
            temperature: 0.7,
            debug_logging: false,
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
    tools: Vec<crate::api::agent::core::types::Tool>,
    tool_registry: Arc<ToolRegistry>,
    sqlite_memory: Arc<SqliteConversationMemory>,
    conversation_id: String,
    config: AgentLoopConfig,
) -> Result<AgentLoopResult> {
    let mut tool_results = Vec::new();
    let mut iterations = 0;
    let logger = ConversationLogger::new(config.debug_logging, &conversation_id);

    logger.log("START", "Agent loop started");
    logger.log("MESSAGES", "Initial message history:");
    for msg in &messages {
        logger.log_message(msg);
    }

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
                    msg.content.text()
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
            stream: Some(false),
            stream_options: None,
        };

        println!("📤 Sending request to LLM (iteration {})...", iterations);
        logger.log(
            "LOOP ITERATION",
            &format!("Sending request to LLM (iteration {})...", iterations),
        );
        let response = client.post(llama_url).json(&request).send().await?;

        let response_status = response.status();
        let response_text = response.text().await?;

        logger.log("LLM RESPONSE RAW", &response_text);

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

            // Store assistant message with tool calls in SQLite
            let assistant_message = choice.message.clone();
            if let Err(e) = sqlite_memory
                .add_message(&conversation_id, assistant_message.clone())
                .await
            {
                println!("⚠️ Failed to store assistant tool call message: {}", e);
            }
            messages.push(assistant_message.clone());
            logger.log_message(&assistant_message);

            // Execute all tool calls in parallel
            let mut futures = Vec::new();
            for tool_call in tool_calls {
                println!(
                    "   📞 Spawning tool execution: {} with args: {}",
                    tool_call.function.name, tool_call.function.arguments
                );

                let registry = tool_registry.clone();
                let call = tool_call.clone();

                futures.push(tokio::spawn(async move {
                    let result = registry.execute_tool_call(&call).await;
                    (call, result)
                }));
            }

            // Wait for all tools to complete
            let results = futures::future::join_all(futures).await;

            // Process results
            let mut iteration_tool_results = Vec::new();

            for join_result in results {
                match join_result {
                    Ok((tool_call, execution_result)) => match execution_result {
                        Ok(result) => {
                            println!(
                                "   ✅ Tool '{}' executed successfully",
                                tool_call.function.name
                            );
                            iteration_tool_results.push((tool_call.clone(), result.clone()));
                            tool_results.push(result.clone());
                            logger.log_tool_result(&result);
                        }
                        Err(e) => {
                            println!("   Tool execution error: {:#}", e);
                            let error_result = ToolCallResult {
                                tool_call_id: None,
                                tool_name: tool_call.function.name.clone(),
                                result: format!("Error: {:#}", e),
                            };
                            iteration_tool_results.push((tool_call, error_result.clone()));
                            tool_results.push(error_result);
                        }
                    },
                    Err(e) => {
                        println!("   Tool task panic: {}", e);
                        // Handle panic if needed, though unlikely
                    }
                }
            }

            // Add tool results as tool messages and store in SQLite
            for (tool_call, result) in iteration_tool_results {
                let tool_message = ChatMessage {
                    role: MessageRole::Tool,
                    content: MessageContent::Text(result.result.clone()),
                    name: Some(tool_call.function.name.clone()),
                    tool_calls: None,
                    tool_call_id: Some(tool_call.id.clone()),
                    reasoning_content: None,
                };

                if let Err(e) = sqlite_memory
                    .add_message(&conversation_id, tool_message.clone())
                    .await
                {
                    println!("⚠️ Failed to store tool result message: {}", e);
                }

                messages.push(tool_message.clone());
                logger.log_message(&tool_message);
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
                choice.message.content.text()
            };

            println!(
                "✅ LLM provided final answer after {} iterations",
                iterations
            );

            // Store final assistant response in memory
            let final_assistant_message = ChatMessage {
                role: MessageRole::Assistant,
                content: MessageContent::Text(final_message.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::{FunctionDefinition, Tool};
    use crate::api::agent::memory::sqlite_memory::new_test_memory;
    use crate::test_support::{
        assistant_completion, tool_call_completion, EchoTool, MockLlm, MockLlmConfig,
        UNREACHABLE_LLM_URL,
    };

    /// The tool name the mock LLM asks for. It is deliberately absent from every
    /// registry these tests build (they are all empty), so the dispatch path is
    /// exercised without any tool - and therefore any outbound request - running.
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

    fn tool_message(name: &str, text: &str) -> ChatMessage {
        ChatMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(text.to_string()),
            name: Some(name.to_string()),
            tool_calls: None,
            tool_call_id: Some(format!("call_{}", name)),
            reasoning_content: None,
        }
    }

    fn tool_definition(name: &str) -> Tool {
        Tool {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: name.to_string(),
                description: "a tool the LLM is told about but never asked to run".to_string(),
                parameters: serde_json::json!({ "type": "object", "properties": {} }),
            },
        }
    }

    /// Fresh in-memory-ish conversation store with one conversation to write into.
    async fn conversation() -> (tempfile::TempDir, Arc<SqliteConversationMemory>, String) {
        let (dir, memory) = new_test_memory().await;
        let memory = Arc::new(memory);
        let id = memory
            .get_or_create_conversation_id(None, Some("test-model"))
            .await
            .expect("Failed to create conversation");
        (dir, memory, id)
    }

    /// Runs the loop against `url` with an empty tool registry.
    #[allow(clippy::too_many_arguments)]
    async fn run_loop(
        url: &str,
        messages: Vec<ChatMessage>,
        tools: Vec<Tool>,
        memory: Arc<SqliteConversationMemory>,
        conversation_id: &str,
        config: AgentLoopConfig,
    ) -> Result<AgentLoopResult> {
        execute_agent_loop(
            &Client::new(),
            url,
            "test-model".to_string(),
            messages,
            tools,
            Arc::new(ToolRegistry::new()),
            memory,
            conversation_id.to_string(),
            config,
        )
        .await
    }

    #[test]
    fn test_default_config_caps_iterations_and_tokens() {
        let config = AgentLoopConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.max_tokens, 2000);
        assert_eq!(config.temperature, 0.7);
        assert!(!config.debug_logging);
    }

    #[tokio::test]
    async fn test_plain_reply_is_returned_and_stored() {
        let llm = MockLlm::start(MockLlmConfig::replying("Rome is the capital.")).await;
        let (_dir, memory, id) = conversation().await;

        let result = run_loop(
            &llm.chat_url(),
            vec![message(MessageRole::User, "What is the capital of Italy?")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect("The loop should succeed");

        assert_eq!(result.final_message, "Rome is the capital.");
        assert_eq!(result.iterations, 1);
        assert!(!result.stuck);
        assert!(result.tool_calls.is_empty());

        // The assistant answer is persisted, and nothing else is
        let stored = memory.get_messages(&id).await.expect("Failed to read back");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].role, MessageRole::Assistant);
        assert_eq!(stored[0].content.text(), "Rome is the capital.");

        // Without tools, the request carries neither a tool list nor a tool_choice
        let requests = llm.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["model"], "test-model");
        assert_eq!(requests[0]["stream"], false);
        assert_eq!(requests[0]["max_tokens"], 2000);
        assert!(requests[0].get("tools").is_none(), "{}", requests[0]);
        assert!(requests[0].get("tool_choice").is_none(), "{}", requests[0]);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_tool_definitions_are_advertised_with_auto_tool_choice() {
        let llm = MockLlm::start(MockLlmConfig::replying("no tool needed")).await;
        let (_dir, memory, id) = conversation().await;

        let result = run_loop(
            &llm.chat_url(),
            vec![message(MessageRole::User, "hi")],
            vec![tool_definition("some_tool")],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect("The loop should succeed");

        assert_eq!(result.final_message, "no tool needed");
        let requests = llm.requests();
        assert_eq!(requests[0]["tool_choice"], "auto");
        assert_eq!(requests[0]["tools"][0]["function"]["name"], "some_tool");

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_empty_reply_without_tool_results_falls_back_to_a_placeholder() {
        let llm = MockLlm::start(MockLlmConfig::replying("")).await;
        let (_dir, memory, id) = conversation().await;

        let result = run_loop(
            &llm.chat_url(),
            vec![message(MessageRole::User, "hi")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect("The loop should succeed");

        assert_eq!(result.final_message, "I've processed your request.");
        let stored = memory.get_messages(&id).await.expect("Failed to read back");
        assert_eq!(stored[0].content.text(), "I've processed your request.");

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_tool_role_messages_are_replayed_as_user_messages() {
        let llm = MockLlm::start(MockLlmConfig::replying("understood")).await;
        let (_dir, memory, id) = conversation().await;

        run_loop(
            &llm.chat_url(),
            vec![
                message(MessageRole::System, "you are helpful"),
                message(MessageRole::User, "weather?"),
                tool_message("weather", "sunny"),
                message(MessageRole::User, "and tomorrow?"),
                tool_message("forecast", "rain"),
            ],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect("The loop should succeed");

        // Tool messages are folded into user messages, both mid-history and at the
        // very end, so the server only ever sees system/user/assistant turns.
        let sent = &llm.requests()[0]["messages"];
        let roles: Vec<&str> = sent
            .as_array()
            .expect("messages should be an array")
            .iter()
            .map(|m| m["role"].as_str().expect("role should be a string"))
            .collect();
        assert_eq!(roles, vec!["system", "user", "user", "user", "user"]);
        assert_eq!(sent[2]["content"], "Tool results:\nweather: sunny");
        assert_eq!(sent[3]["content"], "and tomorrow?");
        assert_eq!(sent[4]["content"], "Tool results:\nforecast: rain");

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_requested_tool_calls_are_dispatched_and_their_results_fed_back() {
        let llm = MockLlm::start(MockLlmConfig::replying_with_bodies(vec![
            tool_call_completion("call_1", MISSING_TOOL, "{}"),
            assistant_completion("done"),
        ]))
        .await;
        let (_dir, memory, id) = conversation().await;

        let result = run_loop(
            &llm.chat_url(),
            vec![message(MessageRole::User, "use a tool")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect("The loop should succeed");

        assert_eq!(result.final_message, "done");
        assert_eq!(result.iterations, 2);
        assert!(!result.stuck);

        // The registry is empty, so the dispatch fails and the failure is reported
        // back to the model as a tool result rather than aborting the loop.
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].tool_name, MISSING_TOOL);
        assert!(
            result.tool_calls[0].result.starts_with("Error: Tool"),
            "{}",
            result.tool_calls[0].result
        );

        // Assistant tool-call turn, tool result turn and the final answer are stored
        let stored = memory.get_messages(&id).await.expect("Failed to read back");
        assert_eq!(stored.len(), 3);
        assert_eq!(stored[0].role, MessageRole::Assistant);
        assert_eq!(
            stored[0].tool_calls.as_ref().expect("tool calls")[0]
                .function
                .name,
            MISSING_TOOL
        );
        assert_eq!(stored[1].role, MessageRole::Tool);
        assert_eq!(stored[1].name.as_deref(), Some(MISSING_TOOL));
        assert_eq!(stored[1].tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(stored[2].role, MessageRole::Assistant);
        assert_eq!(stored[2].content.text(), "done");

        // The second request replays the tool result as a user message
        let second = &llm.requests()[1]["messages"];
        let last = second.as_array().expect("array").last().expect("last");
        assert_eq!(
            last["content"],
            serde_json::json!(format!(
                "Tool results:\n{}: {}",
                MISSING_TOOL, result.tool_calls[0].result
            ))
        );

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_successful_tool_result_is_fed_back_to_the_model() {
        let llm = MockLlm::start(MockLlmConfig::replying_with_bodies(vec![
            tool_call_completion("call_1", "echo", "{}"),
            assistant_completion("The echo said hello."),
        ]))
        .await;
        let (_dir, memory, id) = conversation().await;

        // `EchoTool` does no I/O whatsoever, so it is safe to let the loop run it
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(EchoTool::new("echo", "hello")))
            .expect("Failed to register the echo tool");

        let result = execute_agent_loop(
            &Client::new(),
            &llm.chat_url(),
            "test-model".to_string(),
            vec![message(MessageRole::User, "make it echo")],
            registry
                .build_tool_definitions()
                .expect("Failed to build tool definitions"),
            Arc::new(registry),
            Arc::clone(&memory),
            id.clone(),
            AgentLoopConfig::default(),
        )
        .await
        .expect("The loop should succeed");

        assert_eq!(result.final_message, "The echo said hello.");
        assert_eq!(result.iterations, 2);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].tool_name, "echo");
        assert_eq!(result.tool_calls[0].result, "hello");
        assert_eq!(result.tool_calls[0].tool_call_id.as_deref(), Some("call_1"));

        // The result is stored as a tool turn and replayed to the model
        let stored = memory.get_messages(&id).await.expect("Failed to read back");
        assert_eq!(stored.len(), 3);
        assert_eq!(stored[1].role, MessageRole::Tool);
        assert_eq!(stored[1].content.text(), "hello");
        let second = &llm.requests()[1]["messages"];
        assert_eq!(
            second.as_array().expect("array").last().expect("last")["content"],
            "Tool results:\necho: hello"
        );

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_empty_reply_after_a_tool_call_reports_gathered_information() {
        let llm = MockLlm::start(MockLlmConfig::replying_with_bodies(vec![
            tool_call_completion("call_1", MISSING_TOOL, "{}"),
            assistant_completion(""),
        ]))
        .await;
        let (_dir, memory, id) = conversation().await;

        let result = run_loop(
            &llm.chat_url(),
            vec![message(MessageRole::User, "use a tool")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect("The loop should succeed");

        assert_eq!(
            result.final_message,
            "I've gathered the requested information."
        );
        assert_eq!(result.tool_calls.len(), 1);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_model_that_only_ever_calls_tools_is_reported_as_stuck() {
        let llm = MockLlm::start(MockLlmConfig::replying_with_bodies(vec![
            tool_call_completion("call_1", MISSING_TOOL, "{}"),
        ]))
        .await;
        let (_dir, memory, id) = conversation().await;

        let result = run_loop(
            &llm.chat_url(),
            vec![message(MessageRole::User, "loop forever")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig {
                max_iterations: 2,
                ..Default::default()
            },
        )
        .await
        .expect("The loop should give up rather than error");

        assert!(result.stuck);
        assert_eq!(result.iterations, 3, "one iteration past the cap");
        assert_eq!(llm.call_count(), 2, "the cap is checked before requesting");
        assert_eq!(result.tool_calls.len(), 2);
        // The last assistant turn is a tool call with no text, so a placeholder is used
        assert_eq!(
            result.final_message,
            "I've gathered information but reached the maximum number of iterations. Here's what I found."
        );

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_stuck_loop_reuses_the_last_assistant_text_when_there_is_some() {
        let (_dir, memory, id) = conversation().await;

        // max_iterations 0 trips the cap before the first request, so no server is needed
        let result = run_loop(
            UNREACHABLE_LLM_URL,
            vec![
                message(MessageRole::User, "hi"),
                message(MessageRole::Assistant, "a partial answer"),
            ],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig {
                max_iterations: 0,
                ..Default::default()
            },
        )
        .await
        .expect("The loop should give up rather than error");

        assert!(result.stuck);
        assert_eq!(result.final_message, "a partial answer");
        assert!(
            memory
                .get_messages(&id)
                .await
                .expect("Failed to read back")
                .is_empty(),
            "giving up must not persist anything"
        );
    }

    #[tokio::test]
    async fn test_stuck_loop_without_any_assistant_turn_uses_a_generic_message() {
        let (_dir, memory, id) = conversation().await;

        let result = run_loop(
            UNREACHABLE_LLM_URL,
            vec![message(MessageRole::User, "hi")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig {
                max_iterations: 0,
                ..Default::default()
            },
        )
        .await
        .expect("The loop should give up rather than error");

        assert!(result.stuck);
        assert_eq!(
            result.final_message,
            "I've processed your request but reached the maximum number of iterations."
        );
    }

    #[tokio::test]
    async fn test_stuck_loop_ignores_an_empty_last_assistant_turn() {
        let (_dir, memory, id) = conversation().await;

        let result = run_loop(
            UNREACHABLE_LLM_URL,
            vec![
                message(MessageRole::Assistant, "earlier text"),
                message(MessageRole::Assistant, ""),
            ],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig {
                max_iterations: 0,
                ..Default::default()
            },
        )
        .await
        .expect("The loop should give up rather than error");

        assert!(result.stuck);
        assert_eq!(
            result.final_message,
            "I've gathered information but reached the maximum number of iterations. Here's what I found."
        );
    }

    #[tokio::test]
    async fn test_a_server_error_status_aborts_the_loop() {
        let mut config = MockLlmConfig::replying("never read");
        config.chat_status = 503;
        let llm = MockLlm::start(config).await;
        let (_dir, memory, id) = conversation().await;

        let err = run_loop(
            &llm.chat_url(),
            vec![message(MessageRole::User, "hi")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect_err("A 503 should surface as an error");

        let err = err.to_string();
        assert!(err.contains("LLM server error"), "{}", err);
        assert!(err.contains("503"), "{}", err);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_an_unparsable_body_aborts_the_loop() {
        let mut config = MockLlmConfig::replying("ignored");
        config.chat_body = "<html>not json</html>".to_string();
        let llm = MockLlm::start(config).await;
        let (_dir, memory, id) = conversation().await;

        let err = run_loop(
            &llm.chat_url(),
            vec![message(MessageRole::User, "hi")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect_err("Garbage should surface as an error");

        let err = err.to_string();
        assert!(err.contains("Failed to parse LLM response"), "{}", err);
        assert!(err.contains("<html>not json</html>"), "{}", err);

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_a_response_without_choices_aborts_the_loop() {
        let mut config = MockLlmConfig::replying("ignored");
        config.chat_body = serde_json::json!({
            "id": "test",
            "object": "chat.completion",
            "created": 0,
            "model": "test-model",
            "choices": []
        })
        .to_string();
        let llm = MockLlm::start(config).await;
        let (_dir, memory, id) = conversation().await;

        let err = run_loop(
            &llm.chat_url(),
            vec![message(MessageRole::User, "hi")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect_err("An empty choice list should surface as an error");

        assert!(
            err.to_string().contains("No choices in LLM response"),
            "{}",
            err
        );

        llm.stop().await;
    }

    #[tokio::test]
    async fn test_an_unreachable_server_aborts_the_loop() {
        let (_dir, memory, id) = conversation().await;

        let err = run_loop(
            &format!("{}/v1/chat/completions", UNREACHABLE_LLM_URL),
            vec![message(MessageRole::User, "hi")],
            vec![],
            Arc::clone(&memory),
            &id,
            AgentLoopConfig::default(),
        )
        .await
        .expect_err("A refused connection should surface as an error");

        assert!(
            err.to_string().contains("error sending request")
                || err.to_string().contains("Connection refused"),
            "{}",
            err
        );
    }
}
