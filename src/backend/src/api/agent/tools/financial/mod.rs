pub mod currency;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::financial::currency::CurrencyTool;
use crate::api::agent::tools::framework::registry::ToolRegistry;
use std::sync::Arc;

pub fn register(registry: &mut ToolRegistry, config: &AgentConfig) {
    if config.enabled_tools.contains(&ToolType::Currency) {
        let tool = CurrencyTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Currency tool: {}", e);
        }
    }
}
