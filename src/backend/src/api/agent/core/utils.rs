use crate::api::agent::tools::framework::agent_tool::{ToolCategory, ToolMetadata};

pub enum StatusType {
    Calling,
    Executing,
    Complete(std::time::Duration),
    Error(std::time::Duration),
}

pub fn format_tool_status_message(
    name: &str,
    metadata: Option<&ToolMetadata>,
    status_type: StatusType,
) -> String {
    let category = metadata
        .map(|m| m.category)
        .unwrap_or(ToolCategory::Utility);

    match status_type {
        StatusType::Calling => match category {
            ToolCategory::Web => format!("Browsing {}...", name),
            ToolCategory::Financial => format!("Consulting {}...", name),
            ToolCategory::Database => format!("Querying {}...", name),
            ToolCategory::Search => format!("Searching {}...", name),
            ToolCategory::Development => format!("Preparing {}...", name),
            _ => format!("Calling {}...", name),
        },
        StatusType::Executing => match category {
            ToolCategory::Web => format!("Visiting {}...", name),
            ToolCategory::Financial => "Analyzing market data...".to_string(),
            ToolCategory::Database => "Searching knowledge base...".to_string(),
            ToolCategory::Search => "Scanning web results...".to_string(),
            ToolCategory::Development => "Executing code...".to_string(),
            _ => format!("Executing {}...", name),
        },
        StatusType::Complete(duration) => {
            let time_str = format!("({:.1}s)", duration.as_secs_f64());
            match category {
                ToolCategory::Web => format!("Visited {} {}", name, time_str),
                ToolCategory::Financial => format!("Market data retrieved {}", time_str),
                ToolCategory::Database => format!("Found relevant info {}", time_str),
                ToolCategory::Search => format!("Search completed {}", time_str),
                ToolCategory::Development => format!("Code execution finished {}", time_str),
                _ => format!("{} completed {}", name, time_str),
            }
        }
        StatusType::Error(duration) => {
            format!("{} failed after {:.1}s", name, duration.as_secs_f64())
        }
    }
}
