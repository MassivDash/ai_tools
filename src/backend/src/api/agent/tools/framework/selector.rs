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

            // If PageIndex is enabled, explain the vectorless, reasoning-based navigation strategy
            let has_pageindex_tool = all_tools
                .iter()
                .any(|tool| tool.metadata().tool_type == ToolType::PageIndex);

            if has_pageindex_tool {
                prompt.push_str("**PAGEINDEX (REASONING-BASED BOOK NAVIGATION):** You have access to hierarchical table-of-contents indexes for technical books, built without vector search. To use it: call `pageindex_tool` with action `get_tree` first to see a book's chapter/section outline and per-section summaries.\n");
                prompt.push_str("- **Reason, Don't Match Keywords:** Read the section summaries and pick the node(s) most likely to answer the question by reasoning about their meaning, not by matching keywords.\n");
                prompt.push_str("- **Drill Down When Needed:** If a summary is too broad to confidently answer, call `get_tree` again (or use the children already returned) and pick a more specific child node instead of guessing.\n");
                prompt.push_str("- **Read Before Answering:** Once you've identified the relevant node, call `pageindex_tool` with action `read_node` to fetch its actual text before answering - never answer from the summary alone.\n\n");
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
```
- CHART COLORS: Each series may optionally include a `color` field (a hex string, e.g. \"#e34948\"). Leave it out by default — the chart automatically assigns a distinct, accessible color to each series. Only set `color` explicitly when the user asks for specific/custom colors, or when a color carries meaning (e.g. red for losses, green for gains).",
        );

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::EchoTool;

    /// A registry holding one echo tool per `(id, tool_type)` pair.
    fn registry_with(tools: Vec<(&str, ToolType)>) -> Arc<ToolRegistry> {
        let mut registry = ToolRegistry::new();
        for (name, tool_type) in tools {
            registry
                .register(Arc::new(
                    EchoTool::new(name, "canned").with_tool_type(tool_type),
                ))
                .expect("Registering a test tool should succeed");
        }
        Arc::new(registry)
    }

    #[test]
    fn an_empty_registry_says_no_tools_are_enabled() {
        let prompt = ToolSelector::new(Arc::new(ToolRegistry::new())).build_system_prompt();

        assert!(prompt.starts_with("Current Date/Time: "));
        assert!(prompt.contains("User Default Timezone: Europe/Warsaw"));
        assert!(prompt.contains("AVAILABLE TOOLS: None (no tools are currently enabled)"));
        // The tool-specific preambles are all absent.
        assert!(!prompt.contains("**KNOWLEDGE BASE & RAG ALWAYS:**"));
        assert!(!prompt.contains("**PAGEINDEX"));
        assert!(!prompt.contains("THINK FIRST"));
        // The general guidelines are always appended.
        assert!(prompt.contains("GUIDELINES:"));
        assert!(prompt.contains("```json-chart"));
    }

    #[test]
    fn registered_tools_are_numbered_with_their_function_descriptions() {
        let prompt = ToolSelector::new(registry_with(vec![
            ("first_tool", ToolType::AskHuman),
            ("second_tool", ToolType::AskHuman),
        ]))
        .build_system_prompt();

        assert!(prompt.contains("AVAILABLE TOOLS:\n"));
        assert!(prompt.contains("THINK FIRST BEFORE USING ANY OTHER TOOLS"));
        // The registry is a HashMap, so only the numbering and the presence of
        // both entries is guaranteed, not their order.
        assert!(prompt.contains("1. "));
        assert!(prompt.contains("2. "));
        assert!(prompt.contains("first_tool: A test tool that echoes a canned result"));
        assert!(prompt.contains("second_tool: A test tool that echoes a canned result"));
        // Neither RAG preamble applies to these tool types.
        assert!(!prompt.contains("**KNOWLEDGE BASE & RAG ALWAYS:**"));
        assert!(!prompt.contains("**PAGEINDEX"));
    }

    #[test]
    fn a_chromadb_tool_adds_the_knowledge_base_instructions() {
        let prompt = ToolSelector::new(registry_with(vec![("kb", ToolType::ChromaDB)]))
            .build_system_prompt();

        assert!(prompt.contains("**KNOWLEDGE BASE & RAG ALWAYS:**"));
        assert!(prompt.contains("**Multi-Query Strategy:**"));
        assert!(prompt.contains("**Deep Dive (REQUIRED):**"));
        assert!(prompt.contains("**Cite Sources:**"));
        assert!(!prompt.contains("**PAGEINDEX"));
    }

    #[test]
    fn a_pageindex_tool_adds_the_reasoning_navigation_instructions() {
        let prompt = ToolSelector::new(registry_with(vec![("outline", ToolType::PageIndex)]))
            .build_system_prompt();

        assert!(prompt.contains("**PAGEINDEX (REASONING-BASED BOOK NAVIGATION):**"));
        assert!(prompt.contains("**Reason, Don't Match Keywords:**"));
        assert!(prompt.contains("**Read Before Answering:**"));
        assert!(!prompt.contains("**KNOWLEDGE BASE & RAG ALWAYS:**"));
    }

    #[test]
    fn both_rag_preambles_appear_when_both_tool_types_are_enabled() {
        let prompt = ToolSelector::new(registry_with(vec![
            ("kb", ToolType::ChromaDB),
            ("outline", ToolType::PageIndex),
        ]))
        .build_system_prompt();

        let knowledge_base = prompt
            .find("**KNOWLEDGE BASE & RAG ALWAYS:**")
            .expect("the knowledge base preamble");
        let pageindex = prompt
            .find("**PAGEINDEX (REASONING-BASED BOOK NAVIGATION):**")
            .expect("the pageindex preamble");
        let think_first = prompt.find("**THINK FIRST").expect("the think-first line");
        // Both preambles precede the general "think first" warning.
        assert!(knowledge_base < pageindex, "{}", prompt);
        assert!(pageindex < think_first, "{}", prompt);
    }
}
