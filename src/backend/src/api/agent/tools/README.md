# Writing Tools for the Agent

This guide explains how to create new tools for the agent system.

## Overview

Tools are modular components that extend the agent's capabilities. Each tool implements the `AgentTool` trait and can be registered with the `ToolRegistry` to make it available to the LLM.

The tool system follows this architecture:
- **AgentTool Trait**: Defines the interface all tools must implement
- **ToolRegistry**: Manages registration and lookup of tools
- **ToolSelector**: Builds system prompts with available tool information
- **Dynamic Registration**: Tools are registered per-request based on configuration

## Tool Structure

A tool consists of:
1. **Metadata** - ID and name for identification
2. **Function Definition** - OpenAI-compatible function schema (JSON Schema)
3. **Execution Logic** - The actual async tool implementation
4. **Availability Check** - Optional method to verify tool can be used

## Architecture Components

### AgentTool Trait

All tools must implement the `AgentTool` trait from `agent_tool.rs`:

```rust
#[async_trait]
pub trait AgentTool: Send + Sync {
    fn metadata(&self) -> &ToolMetadata;
    fn get_function_definition(&self) -> serde_json::Value;
    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult>;
    fn is_available(&self) -> bool { true } // Optional override
}
```

### ToolRegistry

The `ToolRegistry` manages all registered tools:
- Stores tools in a `HashMap<String, Arc<dyn AgentTool>>`
- Checks `is_available()` before registering
- Finds tools by ID or function name
- Builds OpenAI-compatible tool definitions
- Thread-safe (uses `Arc`)

### ToolSelector

The `ToolSelector` builds system prompts that inform the LLM about available tools:
- Only includes tools that are currently registered/enabled
- Formats tool descriptions for the LLM
- Provides usage guidelines

## Step-by-Step Guide

### 1. Create a New Tool File

Create a new file in `src/backend/src/api/agent/tools/` (e.g., `my_tool.rs`):

```rust
use crate::api::agent::tools::agent_tool::{AgentTool, ToolMetadata};
use crate::api::agent::types::{ToolCall, ToolCallResult};
use anyhow::{Context, Result};
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
                id: "unique_id".to_string(), // Must be unique across all tools
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
            "description": "Clear description of what this tool does and when to use it. Be specific about use cases.",
            "parameters": {
                "type": "object",
                "properties": {
                    "param_name": {
                        "type": "string", // or "integer", "boolean", "array", "object"
                        "description": "What this parameter does"
                    }
                },
                "required": ["param_name"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        // Parse arguments from JSON string
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .context("Failed to parse tool call arguments")?;

        let param_value = args
            .get("param_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: param_name"))?;

        // Your tool logic here
        println!("🔧 Executing tool with: {}", param_value);
        let result = format!("Tool executed with: {}", param_value);

        Ok(ToolCallResult {
            tool_name: "function_name".to_string(), // Must match function definition name
            result,
        })
    }

    // Optional: Override is_available() if your tool has dependencies
    fn is_available(&self) -> bool {
        true // Return false if tool can't be used (e.g., missing config, service down)
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
    WebsiteCheck,
    MyTool, // Add your tool here
}
```

### 4. Register Tool in Chat Endpoints

In `src/backend/src/api/agent/chat.rs`, add registration logic in **both** `agent_chat` and `agent_chat_stream` functions:

```rust
// Register your tool if enabled
if config.enabled_tools.contains(&ToolType::MyTool) {
    let my_tool = MyTool::new();
    if let Err(e) = tool_registry.register(Arc::new(my_tool)) {
        println!("⚠️ Failed to register My Tool: {}", e);
    }
}
```

**Important**: ChromaDB is a special case - it's registered based on `config.chromadb` (not `enabled_tools`), and requires a configuration object.

### 5. Import Your Tool

Add the import at the top of `chat.rs`:

```rust
use crate::api::agent::tools::{
    chromadb::ChromaDBTool,
    financial_data::FinancialDataTool,
    website_check::WebsiteCheckTool,
    my_tool::MyTool, // Add your tool
    registry::ToolRegistry,
    selector::ToolSelector,
};
```

### 6. (Optional) Add Tool Metadata for Frontend

If you want your tool to appear in the frontend tool list, add it to the `/api/agent/tools` endpoint in `src/backend/src/api/agent/config.rs`:

```rust
ToolInfo {
    id: "unique_id".to_string(), // Must match your tool's metadata.id
    name: "human readable name".to_string(), // Must match your tool's metadata.name
    tool_type: ToolType::MyTool,
    description: "Tool description for UI".to_string(),
    category: ToolCategory::Utility, // Choose appropriate category
    icon: ToolCategory::Utility.icon_name().to_string(),
},
```

## Best Practices

### Function Definitions

- **Name**: Use clear, descriptive function names (e.g., `check_website`, `search_chromadb`, `get_financial_data`)
  - Use snake_case for consistency
  - Should clearly indicate what the tool does
- **Description**: Be specific about when to use the tool
  - Include examples of use cases
  - Mention when NOT to use the tool (e.g., "DO NOT use for casual greetings")
  - Guide the LLM on appropriate usage
- **Parameters**: 
  - Use clear, descriptive parameter names
  - Provide helpful descriptions for each parameter
  - Mark required vs optional parameters correctly
  - Use appropriate JSON Schema types (string, integer, boolean, array, object)
  - Add constraints like `minimum`, `maximum`, `enum` when applicable
  - Consider default values in descriptions for optional parameters

### Error Handling

- Always use `anyhow::Result` for error handling
- Use `.context()` to add meaningful error context at each step
- Parse arguments with proper error messages
- Validate inputs before processing
- Return meaningful error messages in `ToolCallResult` (the LLM will see these)
- Use `println!` with emojis for console logging (✅, ⚠️, 🔍, 🌐, etc.)

### Tool Availability

- Override `is_available()` if your tool requires external dependencies
- Tools that aren't available will be skipped during registration (no error thrown)
- Check for required configuration, network connectivity, or service availability
- Example: ChromaDB tool checks if client can be created

### Tool IDs

- Use unique IDs across all tools (e.g., "1", "2", "3" or descriptive IDs like "website_check")
- IDs are used internally for registration tracking and lookup
- Current IDs: ChromaDB="1", FinancialData="2", WebsiteCheck="3"

### Thread Safety

- All tools must be `Send + Sync` (required by the trait)
- Use `Arc<dyn AgentTool>` when sharing tools
- Avoid mutable state or use proper synchronization (Mutex, RwLock) if needed

### Console Logging

The codebase uses emoji-prefixed console logs for better visibility:
- ✅ Success messages
- ⚠️ Warnings/errors
- 🔍 Debug/search operations
- 🌐 Network/web operations
- 📦 Registry operations

## Example Tools

### Website Check Tool (`website_check.rs`)

A complete example that:
- Fetches websites via HTTP
- Validates URLs
- Converts HTML to markdown using existing utilities
- Handles size limits (10MB max)
- Returns formatted results with metadata
- Uses proper error handling and logging

Key features:
- Simple tool with no external dependencies
- Reuses `convert_html_to_markdown` utility
- Validates input (URL format, HTTP status)
- Limits resource usage (max HTML size)

### ChromaDB Tool (`chromadb.rs`)

A more complex example that:
- Requires external configuration (ChromaDB address, collection, embedding model)
- Creates a client connection
- Performs semantic search queries
- Filters results by similarity threshold
- Formats multiple document results

Key features:
- Requires configuration object (`ChromaDBToolConfig`)
- Registered separately from `enabled_tools` (uses `config.chromadb`)
- Handles vector similarity calculations
- Returns formatted document batches

### Financial Data Tool (`financial_data.rs`)

A simple mock tool that:
- Returns hardcoded financial data
- Demonstrates minimal tool structure
- Shows enum parameter usage

## Tool Registration Flow

1. **Request arrives** at `agent_chat` or `agent_chat_stream`
2. **Tool registry created** - `ToolRegistry::new()`
3. **Tools registered conditionally**:
   - ChromaDB: if `config.chromadb` is `Some(...)`
   - Other tools: if `config.enabled_tools.contains(&ToolType::...)`
4. **Registry wrapped in Arc** for thread safety
5. **Tool definitions built** - `build_tool_definitions()` creates OpenAI-compatible format
6. **Tool selector created** - builds system prompt with available tools
7. **Tools passed to LLM** via function calling API

## Testing

1. **Start the backend server**
2. **Enable your tool** in the agent configuration (via `/api/agent/config` endpoint)
3. **Test tool registration**:
   - Check console logs for "✅ Registered tool: ..." messages
   - Verify tool appears in registry count
4. **Test tool execution**:
   - Ask the agent a question that should trigger your tool
   - Check console logs for execution messages
   - Verify tool results are returned correctly
5. **Test error handling**:
   - Try invalid inputs
   - Check that errors are handled gracefully
   - Verify error messages are meaningful

## Important Notes

- **Dynamic Registration**: Tools are registered per-request based on configuration
- **ChromaDB Special Case**: Requires separate `chromadb` config (not just `enabled_tools`)
- **Thread Safety**: All tools must be `Send + Sync` (use `Arc` when sharing)
- **Async Execution**: Tools execute asynchronously (`async fn execute`)
- **Function Name Matching**: The `tool_name` in `ToolCallResult` must match the function definition `name`
- **Argument Parsing**: Tool call arguments come as a JSON string in `tool_call.function.arguments`
- **Tool Availability**: Unavailable tools are silently skipped during registration
- **Registry Lookup**: Tools can be found by ID (`get_tool`) or function name (`get_tool_by_name`)

## Tool Categories (Optional)

Tools can optionally be categorized for frontend display using `ToolCategory`:
- `Web` - Web/Internet related tools
- `Financial` - Financial/Money related tools  
- `Database` - Database/Storage related tools
- `Search` - Search/Query related tools
- `File` - File operations
- `Communication` - Communication tools
- `Development` - Development/Code tools
- `Utility` - General/Utility tools

Categories are used in the `/api/agent/tools` endpoint for frontend display and include Material Icon names.

