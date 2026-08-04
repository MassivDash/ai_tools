pub mod chromadb;
pub mod pageindex;

use crate::api::agent::core::types::AgentConfig;
use crate::api::agent::core::types::ToolType;
use crate::api::agent::tools::database::chromadb::ChromaDBTool;
use crate::api::agent::tools::database::pageindex::PageIndexTool;
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

    if config.enabled_tools.contains(&ToolType::PageIndex) {
        let tool = PageIndexTool::new(context.available_page_indexes.to_vec());
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register PageIndex tool: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::tools::RegisterContext;
    use crate::test_support::lock_chroma_endpoint;

    fn context<'a>(chroma_address: Option<&'a str>) -> RegisterContext<'a> {
        RegisterContext {
            chroma_address,
            available_collections: &[],
            available_page_indexes: &[],
        }
    }

    fn config(enabled_tools: Vec<ToolType>) -> AgentConfig {
        AgentConfig {
            enabled_tools,
            ..Default::default()
        }
    }

    /// Registering the ChromaDB tool only builds a client; it never talks to the
    /// server, so a dead loopback address is enough.
    #[test]
    fn test_chromadb_tool_is_registered_when_enabled_and_addressed() {
        let mut registry = ToolRegistry::new();

        {
            let _guard = lock_chroma_endpoint();
            register(
                &mut registry,
                &config(vec![ToolType::ChromaDB]),
                &context(Some("http://127.0.0.1:1")),
            );
        }

        assert!(registry.get_tool_by_name("chromadb_tool").is_some());
        assert!(registry.get_tool_by_name("pageindex_tool").is_none());
    }

    #[test]
    fn test_chromadb_tool_is_skipped_when_no_address_is_configured() {
        let mut registry = ToolRegistry::new();

        register(
            &mut registry,
            &config(vec![ToolType::ChromaDB]),
            &context(None),
        );

        assert!(registry.get_all_tool_ids().is_empty());
    }

    #[test]
    fn test_chromadb_tool_is_skipped_when_not_enabled() {
        let mut registry = ToolRegistry::new();

        register(
            &mut registry,
            &config(vec![ToolType::PageIndex]),
            &context(Some("http://127.0.0.1:1")),
        );

        assert!(registry.get_tool_by_name("chromadb_tool").is_none());
        assert!(registry.get_tool_by_name("pageindex_tool").is_some());
    }

    /// The ChromaDB tool and the PageIndex tool claim different registry ids, so
    /// enabling both registers both.
    #[test]
    fn test_both_database_tools_can_be_registered_together() {
        let mut registry = ToolRegistry::new();

        {
            let _guard = lock_chroma_endpoint();
            register(
                &mut registry,
                &config(vec![ToolType::ChromaDB, ToolType::PageIndex]),
                &context(Some("http://127.0.0.1:1")),
            );
        }

        assert_eq!(registry.get_all_tool_ids().len(), 2);
        assert!(registry.get_tool_by_name("chromadb_tool").is_some());
        assert!(registry.get_tool_by_name("pageindex_tool").is_some());
    }

    /// A duplicate registration is reported and swallowed rather than panicking.
    #[test]
    fn test_registering_the_same_tools_twice_is_survivable() {
        let mut registry = ToolRegistry::new();
        let config = config(vec![ToolType::ChromaDB, ToolType::PageIndex]);

        {
            let _guard = lock_chroma_endpoint();
            register(&mut registry, &config, &context(Some("http://127.0.0.1:1")));
            register(&mut registry, &config, &context(Some("http://127.0.0.1:1")));
        }

        assert_eq!(registry.get_all_tool_ids().len(), 2);
    }

    #[test]
    fn test_no_database_tools_are_registered_when_none_are_enabled() {
        let mut registry = ToolRegistry::new();

        register(
            &mut registry,
            &config(vec![]),
            &context(Some("http://127.0.0.1:1")),
        );

        assert!(registry.get_all_tool_ids().is_empty());
    }
}
