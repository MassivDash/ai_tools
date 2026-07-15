pub mod chromadb;

use crate::api::agent::core::types::AgentConfig;
use crate::api::agent::core::types::ToolType;
use crate::api::agent::tools::database::chromadb::ChromaDBTool;
use crate::api::agent::tools::framework::registry::ToolRegistry;
use std::sync::Arc;

pub fn register(
    registry: &mut ToolRegistry,
    config: &AgentConfig,
    context: &crate::api::agent::tools::RegisterContext,
) {
    if config.enabled_tools.contains(&ToolType::ChromaDB) {
        if let Some(address) = context.chroma_address {
            match ChromaDBTool::new(address, context.available_collections.to_vec()) {
                Ok(tool) => {
                    if let Err(e) = registry.register(Arc::new(tool)) {
                        println!("⚠️ Failed to register ChromaDB tool: {}", e);
                    }
                }
                Err(e) => {
                    println!("⚠️ Failed to create ChromaDB tool: {}", e);
                }
            }
        }
    }
}
