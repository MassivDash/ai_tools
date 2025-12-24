pub mod currency;
pub mod stock;

use crate::api::agent::core::types::{AgentConfig, ToolType};
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
