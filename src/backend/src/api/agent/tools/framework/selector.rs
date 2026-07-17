use crate::api::agent::core::types::ToolType;
use crate::api::agent::tools::framework::registry::ToolRegistry;
use chrono::Utc;
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
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
        let mut prompt = format!(
            "Current Date/Time: {}\nUser Default Timezone: Europe/Warsaw\nYou are a helpful AI assistant with access to tools.\n\n",
            now
        );

        // Get all tools from the registry - this only includes enabled/active tools
        // since the registry is built with only enabled tools in the chat endpoint
        let all_tools = self.registry.get_all_tools();
        if !all_tools.is_empty() {
            prompt.push_str("AVAILABLE TOOLS:\n");

            // If knowledge base (chromadb) is enabled, let the llm know to prioritize them and list sources
            let has_rag_tools = all_tools
                .iter()
                .any(|tool| tool.metadata().tool_type == ToolType::ChromaDB);

            if has_rag_tools {
                prompt.push_str("**KNOWLEDGE BASE & RAG ALWAYS:** You have access to a semantic vector database. ALWAYS search it before answering factual questions.\n");
                prompt.push_str("- **Multi-Query Strategy:** Never rely on just one search. Execute 2-3 different `chromadb_tool` search calls with different phrasing (e.g., one with exact keywords, one with broad concepts) to ensure you find the right information.\n");

                prompt.push_str("- **Deep Dive (REQUIRED):** If your `chromadb_tool` search results show multiple chunks from the exact same `filename`, OR if a chunk has a very small distance score (indicating high relevance), you MUST use the `chromadb_tool` with action `read` to read the full file. Do not rely solely on the snippets if it's highly relevant. Note: If the file is a massive book, the tool will automatically truncate it, so don't worry about context limits.\n");

                prompt.push_str("- **Cite Sources:** When answering based on knowledge base results or documents, you MUST list your sources (e.g., filenames) clearly at the end of your response.\n\n");
            }

            prompt.push_str("**THINK FIRST BEFORE USING ANY OTHER TOOLS THAN THE KNOWLEDGE BASE:** Do you really need to use a tool? If you can answer with your internal knowledge, do NOT use a tool.\n\n");

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
- Respond naturally without explaining tool usage or internal processes
- CRITICAL: When you receive the results of a tool call in your next turn, you MUST use that data to answer the user's question directly. Do NOT simply say 'I have gathered the information' and wait. Synthesize the tool results into a helpful response.
- CHART CREATION: If the user asks for a chart or graph (e.g., timeline, bar chart) to visualize data (like weather forecasts, stock prices, etc.), output the data in a `json-chart` code block strictly following this schema:
```json-chart
{
  \"type\": \"line\", // or \"bar\"
  \"title\": \"Chart Title\",
  \"xAxis\": { \"label\": \"X Axis Label\", \"data\": [\"Label 1\", \"Label 2\"] },
  \"series\": [
    { \"name\": \"Series Name\", \"data\": [10.5, 20.3] }
  ]
}
```",
        );

        prompt
    }
}
