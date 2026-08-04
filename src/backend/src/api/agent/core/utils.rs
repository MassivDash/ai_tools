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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::ToolType;
    use std::time::Duration;

    fn metadata(category: ToolCategory) -> ToolMetadata {
        ToolMetadata {
            id: "some_tool".to_string(),
            name: "some_tool".to_string(),
            tool_type: ToolType::Weather,
            description: "A tool".to_string(),
            category,
        }
    }

    #[test]
    fn test_calling_messages_per_category() {
        let cases = [
            (ToolCategory::Web, "Browsing weather..."),
            (ToolCategory::Financial, "Consulting weather..."),
            (ToolCategory::Database, "Querying weather..."),
            (ToolCategory::Search, "Searching weather..."),
            (ToolCategory::Development, "Preparing weather..."),
            (ToolCategory::Utility, "Calling weather..."),
            (ToolCategory::File, "Calling weather..."),
            (ToolCategory::Google, "Calling weather..."),
            (ToolCategory::Social, "Calling weather..."),
        ];

        for (category, expected) in cases {
            let meta = metadata(category);
            assert_eq!(
                format_tool_status_message("weather", Some(&meta), StatusType::Calling),
                expected,
                "unexpected message for {:?}",
                category
            );
        }
    }

    #[test]
    fn test_executing_messages_per_category() {
        let cases = [
            (ToolCategory::Web, "Visiting weather..."),
            (ToolCategory::Financial, "Analyzing market data..."),
            (ToolCategory::Database, "Searching knowledge base..."),
            (ToolCategory::Search, "Scanning web results..."),
            (ToolCategory::Development, "Executing code..."),
            (ToolCategory::Utility, "Executing weather..."),
            (ToolCategory::Google, "Executing weather..."),
        ];

        for (category, expected) in cases {
            let meta = metadata(category);
            assert_eq!(
                format_tool_status_message("weather", Some(&meta), StatusType::Executing),
                expected,
                "unexpected message for {:?}",
                category
            );
        }
    }

    #[test]
    fn test_complete_messages_include_the_duration() {
        let duration = Duration::from_millis(1250);
        let cases = [
            (ToolCategory::Web, "Visited weather (1.2s)"),
            (ToolCategory::Financial, "Market data retrieved (1.2s)"),
            (ToolCategory::Database, "Found relevant info (1.2s)"),
            (ToolCategory::Search, "Search completed (1.2s)"),
            (ToolCategory::Development, "Code execution finished (1.2s)"),
            (ToolCategory::Utility, "weather completed (1.2s)"),
            (ToolCategory::Social, "weather completed (1.2s)"),
        ];

        for (category, expected) in cases {
            let meta = metadata(category);
            assert_eq!(
                format_tool_status_message("weather", Some(&meta), StatusType::Complete(duration)),
                expected,
                "unexpected message for {:?}",
                category
            );
        }
    }

    #[test]
    fn test_error_message_is_category_independent() {
        let duration = Duration::from_secs(3);

        for category in [ToolCategory::Web, ToolCategory::Database] {
            let meta = metadata(category);
            assert_eq!(
                format_tool_status_message("weather", Some(&meta), StatusType::Error(duration)),
                "weather failed after 3.0s"
            );
        }
    }

    #[test]
    fn test_missing_metadata_falls_back_to_utility_wording() {
        assert_eq!(
            format_tool_status_message("mystery", None, StatusType::Calling),
            "Calling mystery..."
        );
        assert_eq!(
            format_tool_status_message("mystery", None, StatusType::Executing),
            "Executing mystery..."
        );
        assert_eq!(
            format_tool_status_message(
                "mystery",
                None,
                StatusType::Complete(Duration::from_secs_f64(0.44))
            ),
            "mystery completed (0.4s)"
        );
    }
}
