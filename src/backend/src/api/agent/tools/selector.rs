use crate::api::agent::tools::registry::ToolRegistry;
use std::sync::Arc;

/// Tool selector for intelligent tool selection based on queries
pub struct ToolSelector {
    registry: Arc<ToolRegistry>,
}

impl ToolSelector {
    /// Create a new tool selector
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    /// Get a system prompt that includes information about available tools
    /// This only includes tools that are currently registered (i.e., enabled/active)
    pub fn build_system_prompt(&self) -> String {
        let mut prompt = "You are a helpful AI assistant with access to tools.\n\n".to_string();

        // Get all tools from the registry - this only includes enabled/active tools
        // since the registry is built with only enabled tools in the chat endpoint
        let all_tools = self.registry.get_all_tools();
        if !all_tools.is_empty() {
            prompt.push_str("AVAILABLE TOOLS:\n");
            for (i, tool) in all_tools.iter().enumerate() {
                let func_def = tool.get_function_definition();
                let name = func_def
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let description = func_def
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("No description available");

                prompt.push_str(&format!("{}. {}: {}\n\n", i + 1, name, description));
            }
        } else {
            prompt.push_str("AVAILABLE TOOLS: None (no tools are currently enabled)\n\n");
        }

        prompt.push_str(
            "GUIDELINES:
- Use tools iteratively: call tools, analyze results, call again if needed, then provide final answer
- Don't use tools for greetings or small talk
- Respond naturally without explaining tool usage or internal processes",
        );

        prompt
    }
}
