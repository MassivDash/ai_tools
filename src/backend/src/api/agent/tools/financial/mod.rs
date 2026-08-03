pub mod crypto;
pub mod currency;
pub mod stock;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::financial::crypto::CryptoTool;
use crate::api::agent::tools::financial::currency::CurrencyTool;
use crate::api::agent::tools::financial::stock::StockTool;
use crate::api::agent::tools::framework::agent_tool::AgentTool;
use crate::api::agent::tools::framework::registry::ToolRegistry;
use std::sync::Arc;

pub fn register(registry: &mut ToolRegistry, config: &AgentConfig) {
    if config.enabled_tools.contains(&ToolType::Currency) {
        let tool = CurrencyTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Currency tool: {}", e);
        }
    }

    if config.enabled_tools.contains(&ToolType::Crypto) {
        let tool = CryptoTool::new();
        if tool.is_available() {
            if let Err(e) = registry.register(Arc::new(tool)) {
                println!("⚠️ Failed to register Crypto tool: {}", e);
            }
        } else {
            println!("⚠️ Crypto tool unavailable: ALPHA_ADVANTAGE_KEY not set");
        }
    }

    if config.enabled_tools.contains(&ToolType::Stock) {
        let tool = StockTool::new();
        if tool.is_available() {
            if let Err(e) = registry.register(Arc::new(tool)) {
                println!("⚠️ Failed to register Stock tool: {}", e);
            }
        } else {
            println!("⚠️ Stock tool unavailable: ALPHA_ADVANTAGE_KEY not set");
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
    fn each_enabled_tool_is_registered_when_it_reports_itself_available() {
        let mut registry = ToolRegistry::new();
        register(
            &mut registry,
            &config_with(vec![ToolType::Currency, ToolType::Crypto, ToolType::Stock]),
        );

        // Currency needs no credentials, so it is always registered.
        assert!(registry.is_registered("5"), "Currency should be registered");
        // The Alpha Vantage tools are gated on their own availability, which
        // depends on ALPHA_ADVANTAGE_KEY being present in the environment.
        assert_eq!(
            registry.is_registered("7"),
            CryptoTool::new().is_available()
        );
        assert_eq!(registry.is_registered("6"), StockTool::new().is_available());
    }

    #[test]
    fn registering_twice_reports_the_duplicate_without_panicking() {
        let mut registry = ToolRegistry::new();
        let config = config_with(vec![ToolType::Currency]);

        register(&mut registry, &config);
        register(&mut registry, &config);

        // The second pass hits the duplicate-id branch and is simply logged.
        assert_eq!(registry.count(), 1);
        assert!(registry.is_registered("5"));
    }
}
