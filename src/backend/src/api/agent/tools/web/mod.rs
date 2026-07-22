pub mod allegro;
pub mod google_books;
pub mod website_check;

use crate::api::agent::core::types::{AgentConfig, ToolType};
use crate::api::agent::tools::framework::registry::ToolRegistry;
use crate::api::agent::tools::web::allegro::{AllegroSearchTool, AllegroWishlistTool};
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

    if config.enabled_tools.contains(&ToolType::AllegroSearch) {
        let tool = AllegroSearchTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Allegro Search tool: {}", e);
        }
    }

    if config.enabled_tools.contains(&ToolType::AllegroWishlist) {
        let tool = AllegroWishlistTool::new();
        if let Err(e) = registry.register(Arc::new(tool)) {
            println!("⚠️ Failed to register Allegro Wishlist tool: {}", e);
        }
    }
}
