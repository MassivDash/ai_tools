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
