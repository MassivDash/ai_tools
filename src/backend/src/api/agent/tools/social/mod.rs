pub mod bluesky_post;
pub mod facebook_business_pages;
pub mod facebook_common;
pub mod facebook_post;
pub mod facebook_read_comments;
pub mod facebook_read_messages;
pub mod facebook_read_posts;
pub mod facebook_send_message;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::framework::registry::ToolRegistry;
use crate::api::agent::tools::social::bluesky_post::BlueskyPostTool;
use crate::api::agent::tools::social::facebook_business_pages::FacebookBusinessPagesTool;
use crate::api::agent::tools::social::facebook_post::FacebookPostTool;
use crate::api::agent::tools::social::facebook_read_comments::FacebookReadCommentsTool;
use crate::api::agent::tools::social::facebook_read_messages::FacebookReadMessagesTool;
use crate::api::agent::tools::social::facebook_read_posts::FacebookReadPostsTool;
use crate::api::agent::tools::social::facebook_send_message::FacebookSendMessageTool;
use std::sync::Arc;

pub fn register(registry: &mut ToolRegistry, config: &AgentConfig) {
    if config.enabled_tools.contains(&ToolType::BlueskyPost) {
        let tool = BlueskyPostTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Bluesky Post tool: {}", e);
        }
    }

    if config.enabled_tools.contains(&ToolType::FacebookPost) {
        let tool = FacebookPostTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Facebook Post tool: {}", e);
        }
    }

    if config.enabled_tools.contains(&ToolType::FacebookPostsRead) {
        let tool = FacebookReadPostsTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Facebook Read Posts tool: {}", e);
        }
    }

    if config
        .enabled_tools
        .contains(&ToolType::FacebookCommentsRead)
    {
        let tool = FacebookReadCommentsTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Facebook Read Comments tool: {}", e);
        }
    }

    if config
        .enabled_tools
        .contains(&ToolType::FacebookMessagesRead)
    {
        let tool = FacebookReadMessagesTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Facebook Read Messages tool: {}", e);
        }
    }

    if config
        .enabled_tools
        .contains(&ToolType::FacebookMessageSend)
    {
        let tool = FacebookSendMessageTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Facebook Send Message tool: {}", e);
        }
    }

    if config
        .enabled_tools
        .contains(&ToolType::FacebookBusinessPagesRead)
    {
        let tool = FacebookBusinessPagesTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Facebook Business Pages tool: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(tools: Vec<ToolType>) -> AgentConfig {
        AgentConfig {
            enabled_tools: tools,
            ..Default::default()
        }
    }

    /// Every social tool type, paired with the id it registers under.
    fn all_social_tools() -> Vec<(ToolType, &'static str)> {
        vec![
            (ToolType::BlueskyPost, "bluesky_post"),
            (ToolType::FacebookPost, "facebook_post"),
            (ToolType::FacebookPostsRead, "facebook_read_posts"),
            (ToolType::FacebookCommentsRead, "facebook_read_comments"),
            (ToolType::FacebookMessagesRead, "facebook_read_messages"),
            (ToolType::FacebookMessageSend, "facebook_send_message"),
            (
                ToolType::FacebookBusinessPagesRead,
                "facebook_list_business_pages",
            ),
        ]
    }

    #[test]
    fn nothing_is_registered_for_an_empty_configuration() {
        let mut registry = ToolRegistry::new();
        register(&mut registry, &config_with(vec![]));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn every_social_tool_registers_under_its_own_id() {
        // None of these gate on credentials at registration time - a missing
        // token only surfaces when the tool is actually executed.
        let mut registry = ToolRegistry::new();
        let config = config_with(all_social_tools().into_iter().map(|(t, _)| t).collect());

        register(&mut registry, &config);

        for (_, id) in all_social_tools() {
            assert!(registry.is_registered(id), "{} should be registered", id);
        }
        assert_eq!(registry.count(), all_social_tools().len());

        // A second pass hits every duplicate-id branch and is simply logged.
        register(&mut registry, &config);
        assert_eq!(registry.count(), all_social_tools().len());
    }

    #[test]
    fn enabling_one_tool_registers_only_that_tool() {
        let mut registry = ToolRegistry::new();
        register(&mut registry, &config_with(vec![ToolType::FacebookPost]));

        assert_eq!(registry.get_all_tool_ids(), vec!["facebook_post"]);
    }
}
