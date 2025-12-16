# Writing Tools for the Agent

This guide explains how to create new tools for the agent system.

## Overview

Tools are modular components that extend the agent's capabilities. Each tool implements the `AgentTool` trait and can be registered with the `ToolRegistry` to make it available to the LLM.

## Tool Structure

A tool consists of:
1. **Metadata** - ID and name for identification
2. **Function Definition** - OpenAI-compatible function schema
3. **Execution Logic** - The actual tool implementation

## Step-by-Step Guide

### 1. Create a New Tool File

Create a new file in `src/backend/src/api/agent/tools/` (e.g., `my_tool.rs`):

```rust
use crate::api::agent::tools::agent_tool::{AgentTool, ToolMetadata};
use crate::api::agent::types::{ToolCall, ToolCallResult};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// Your tool implementation
pub struct MyTool {
    metadata: ToolMetadata,
    // Add any fields you need (clients, configs, etc.)
}

impl MyTool {
    /// Create a new instance of your tool
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "unique_id".to_string(), // Must be unique
                name: "human readable name".to_string(),
            },
        }
    }
}

#[async_trait]
impl AgentTool for MyTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "function_name", // Must match what LLM will call
            "description": "Clear description of what this tool does and when to use it",
            "parameters": {
                "type": "object",
                "properties": {
                    "param_name": {
                        "type": "string", // or "integer", "boolean", etc.
                        "description": "What this parameter does"
                    }
                },
                "required": ["param_name"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        // Parse arguments
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let param_value = args
            .get("param_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: param_name"))?;

        // Your tool logic here
        let result = format!("Tool executed with: {}", param_value);

        Ok(ToolCallResult {
            tool_name: "function_name".to_string(),
            result,
        })
    }

    // Optional: Override is_available() if your tool has dependencies
    fn is_available(&self) -> bool {
        true // Return false if tool can't be used (e.g., missing config)
    }
}
```

### 2. Register the Tool in mod.rs

Add your tool module to `src/backend/src/api/agent/tools/mod.rs`:

```rust
pub mod my_tool;
```

### 3. Add Tool Type to Enum

Add your tool to the `ToolType` enum in `src/backend/src/api/agent/types.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    ChromaDB,
    FinancialData,
    WebsiteCheck, // Add your tool here
}
```

### 4. Register Tool in Chat Endpoints

In `src/backend/src/api/agent/chat.rs`, add registration logic in both `agent_chat` and `agent_chat_stream` functions:

```rust
// Register your tool if enabled
if config.enabled_tools.contains(&ToolType::WebsiteCheck) {
    let website_tool = WebsiteCheckTool::new();
    if let Err(e) = tool_registry.register(Arc::new(website_tool)) {
        println!("⚠️ Failed to register Website Check tool: {}", e);
    }
}
```

### 5. Import Your Tool

Add the import at the top of `chat.rs`:

```rust
use crate::api::agent::tools::{
    chromadb::ChromaDBTool,
    financial_data::FinancialDataTool,
    website_check::WebsiteCheckTool, // Add your tool
    registry::ToolRegistry,
    selector::ToolSelector,
};
```

## Best Practices

### Function Definitions

- **Name**: Use clear, descriptive function names (e.g., `check_website`, `search_database`)
- **Description**: Be specific about when to use the tool. Include examples of use cases.
- **Parameters**: 
  - Use clear parameter names
  - Provide helpful descriptions
  - Mark required vs optional parameters correctly
  - Use appropriate types (string, integer, boolean, etc.)

### Error Handling

- Always use `anyhow::Result` for error handling
- Provide context with `.context()` for better error messages
- Return meaningful error messages in `ToolCallResult`

### Tool Availability

- Override `is_available()` if your tool requires external dependencies
- Tools that aren't available will be skipped during registration (no error)

### Tool IDs

- Use unique IDs (e.g., "1", "2", "3" or descriptive IDs like "website_check")
- IDs are used internally for registration tracking

## Example: Website Check Tool

See `website_check.rs` for a complete example that:
- Reuses existing functionality (url_to_markdown)
- Handles HTTP requests
- Converts URLs to markdown
- Returns formatted results to the LLM

## Testing

1. Start the backend server
2. Enable your tool in the agent configuration
3. Test by asking the agent to use your tool
4. Check console logs for registration and execution messages

## Notes

- Tools are registered dynamically based on configuration
- ChromaDB is a special case (requires separate config, not just enabled_tools)
- All tools must be thread-safe (use `Arc` when sharing)
- Tools execute asynchronously (use `async fn execute`)

