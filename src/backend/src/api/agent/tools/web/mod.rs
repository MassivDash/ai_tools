pub mod google_books;
pub mod website_check;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::framework::registry::ToolRegistry;
use crate::api::agent::tools::web::google_books::GoogleBooksTool;
use crate::api::agent::tools::web::website_check::WebsiteCheckTool;
use std::sync::Arc;

pub fn register(registry: &mut ToolRegistry, config: &AgentConfig) {
    if config.enabled_tools.contains(&ToolType::WebsiteCheck) {
        let tool = WebsiteCheckTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Website Check tool: {}", e);
        }
    }

    if config.enabled_tools.contains(&ToolType::GoogleBooks) {
        let tool = GoogleBooksTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Google Books tool: {}", e);
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

    #[test]
    fn nothing_is_registered_for_an_empty_configuration() {
        let mut registry = ToolRegistry::new();
        register(&mut registry, &config_with(vec![]));
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn both_web_tools_register_and_a_second_pass_is_a_no_op() {
        let mut registry = ToolRegistry::new();
        let config = config_with(vec![ToolType::WebsiteCheck, ToolType::GoogleBooks]);

        register(&mut registry, &config);
        assert!(registry.is_registered("3"), "Website Check should register");
        assert!(
            registry.is_registered("google_books"),
            "Google Books should register"
        );

        // The second pass hits the duplicate-id branch and is simply logged.
        register(&mut registry, &config);
        assert_eq!(registry.count(), 2);
    }
}
