pub mod database;
pub mod development;
pub mod financial;
pub mod framework;
pub mod utility;
pub mod web;

use crate::api::agent::core::types::AgentConfig;
use crate::api::agent::tools::framework::registry::ToolRegistry;

/// Context for tool registration containing dependencies that aren't in AgentConfig
pub struct RegisterContext<'a> {
    pub chroma_address: Option<&'a str>,
}

/// Register all enabled tools given the configuration
pub fn register_all(registry: &mut ToolRegistry, config: &AgentConfig, context: &RegisterContext) {
    database::register(registry, config, context);
    development::register(registry, config);
    financial::register(registry, config);
    utility::register(registry, config);
    web::register(registry, config);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::{AgentConfig, ToolType};

    #[test]
    fn test_register_all() {
        let mut registry = ToolRegistry::new();
        let config = AgentConfig {
            enabled_tools: vec![ToolType::GitHubPublic, ToolType::Weather],
            ..Default::default()
        };

        let context = RegisterContext {
            chroma_address: None,
        };

        register_all(&mut registry, &config, &context);

        // Check availability - GitHubPublic should be available
        assert!(
            registry.get_tool("github_public").is_some(),
            "GitHubPublic failed to register"
        );
    }
}
