pub mod github;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::development::github::{GitHubAuthenticatedTool, GitHubPublicTool};
use crate::api::agent::tools::framework::registry::ToolRegistry;
use std::sync::Arc;

pub fn register(registry: &mut ToolRegistry, config: &AgentConfig) {
    if config.enabled_tools.contains(&ToolType::GitHubPublic) {
        let tool = GitHubPublicTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register GitHubPublic tool: {}", e);
        }
    }

    if config
        .enabled_tools
        .contains(&ToolType::GitHubAuthenticated)
    {
        let tool = GitHubAuthenticatedTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register GitHubAuthenticated tool: {}", e);
        }
    }
}
