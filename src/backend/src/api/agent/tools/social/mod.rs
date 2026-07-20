pub mod bluesky_post;
pub mod facebook_post;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::framework::registry::ToolRegistry;
use crate::api::agent::tools::social::bluesky_post::BlueskyPostTool;
use crate::api::agent::tools::social::facebook_post::FacebookPostTool;
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
}
