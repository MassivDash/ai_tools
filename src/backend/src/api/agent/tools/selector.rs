use crate::api::agent::tools::agent_tool::AgentTool;
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

    /// Select the best tools for a given query
    /// Returns a list of tools sorted by relevance
    pub fn select_tools(&self, query: &str, max_tools: Option<usize>) -> Vec<Arc<dyn AgentTool>> {
        let max = max_tools.unwrap_or(5); // Default to top 5 tools
        self.registry
            .find_relevant_tools(query, Some(max))
            .into_iter()
            .map(|(tool, _)| tool)
            .collect()
    }

    /// Get a system prompt that includes information about available tools
    pub fn build_system_prompt(&self) -> String {
        let mut prompt = "You are a helpful AI assistant with access to tools.\n\n".to_string();

        let tools = self.registry.get_all_metadata();
        if !tools.is_empty() {
            prompt.push_str("AVAILABLE TOOLS:\n");
            for (i, metadata) in tools.iter().enumerate() {
                let category_str = match metadata.category {
                    crate::api::agent::tools::agent_tool::ToolCategory::Search => "Search",
                    crate::api::agent::tools::agent_tool::ToolCategory::DataQuery => "Data Query",
                    crate::api::agent::tools::agent_tool::ToolCategory::Analysis => "Analysis",
                    crate::api::agent::tools::agent_tool::ToolCategory::Communication => {
                        "Communication"
                    }
                    crate::api::agent::tools::agent_tool::ToolCategory::Other => "Other",
                };
                prompt.push_str(&format!(
                    "{}. {} (ID: {}, Category: {})\n   Description: {}\n   Keywords: {}\n\n",
                    i + 1,
                    metadata.name,
                    metadata.id,
                    category_str,
                    metadata.description,
                    metadata.keywords.join(", ")
                ));
            }
        }

        prompt.push_str(
            "IMPORTANT GUIDELINES FOR TOOL USAGE AND REASONING:

WORKFLOW:
- You work in an iterative loop where you can use tools multiple times to gather information
- Think step by step: What information do I need? What tools can help? What did I learn? Do I need more information?
- You can call tools multiple times in sequence to build a comprehensive understanding
- Only provide your final answer when you have gathered enough information to give a complete, accurate response

TOOL USAGE:
- DO NOT use tools for casual greetings, small talk, or general conversation (e.g., 'hello', 'how are you', 'thanks', etc.)
- ALWAYS use search tools when the user asks about technical topics, programming frameworks/libraries, code examples, documentation, or specific implementations
- Use search tools for questions about specific people, places, events, technical details, or documents that might be in the knowledge base
- You can use tools multiple times if needed - for example, if initial search results are incomplete, search again with different queries
- If search results are partial or unclear, refine your search query and search again to get more complete information

PROCESSING TOOL RESULTS:
- When you receive tool results, carefully analyze them
- If the results are incomplete or you need more specific information, use tools again with refined queries
- Synthesize information from multiple tool calls to build a comprehensive answer
- Only provide your final answer when you have enough information to be helpful and accurate

RESPONSE STYLE:
- Always respond naturally and conversationally - provide information directly without explaining how you found it
- Do not include internal reasoning, tool call formats, or technical details in your responses
- When you have gathered sufficient information through tools, provide a complete, well-structured answer
- If after multiple tool calls you still don't have enough information, provide what you found and acknowledge limitations",
        );

        prompt
    }

    /// Check if a query likely requires tool usage
    pub fn requires_tools(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Common greetings and small talk that don't need tools
        let no_tool_patterns = vec![
            "hello",
            "hi",
            "hey",
            "greetings",
            "how are you",
            "how u doin",
            "thanks",
            "thank you",
            "bye",
            "goodbye",
            "see you",
            "ok",
            "okay",
            "yes",
            "no",
        ];

        // If it's just a greeting, don't use tools
        if no_tool_patterns
            .iter()
            .any(|pattern| query_lower == *pattern || query_lower.starts_with(pattern))
        {
            return false;
        }

        // Check if any tool is relevant to this query
        let relevant_tools = self.registry.find_relevant_tools(&query_lower, Some(1));
        !relevant_tools.is_empty() && relevant_tools[0].1 > 0.3 // Threshold for relevance
    }
}
