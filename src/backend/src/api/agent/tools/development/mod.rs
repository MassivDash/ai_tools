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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::tools::framework::agent_tool::AgentTool;

    fn config_with(tools: Vec<ToolType>) -> AgentConfig {
        AgentConfig {
            enabled_tools: tools,
            ..Default::default()
        }
    }

    #[test]
    fn nothing_is_registered_for_an_empty_configuration() {
        let mut registry = ToolRegistry::new();
        register(&mut registry, &config_with(vec![]));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn each_enabled_tool_is_registered_when_it_reports_itself_available() {
        let mut registry = ToolRegistry::new();
        let config = config_with(vec![ToolType::GitHubPublic, ToolType::GitHubAuthenticated]);

        register(&mut registry, &config);

        // The public tool needs no credentials; the authenticated one is gated on
        // GITHUB_TOKEN being present in the environment.
        assert!(registry.is_registered("github_public"));
        assert_eq!(
            registry.is_registered("github_authenticated"),
            GitHubAuthenticatedTool::new().is_available()
        );

        // A second pass hits the duplicate-id branch and is simply logged.
        let before = registry.count();
        register(&mut registry, &config);
        assert_eq!(registry.count(), before);
    }
}
