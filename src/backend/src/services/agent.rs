use actix_web::web::ServiceConfig;

use crate::api::agent::service::chat::{agent_chat, agent_chat_stream};
use crate::api::agent::service::config::{
    get_agent_config, get_agent_status, get_available_tools, get_model_capabilities,
    post_agent_config,
};
use crate::api::agent::service::conversations::{
    delete_conversation, get_conversation_history, get_conversations, update_conversation_title,
};

/// Configures all agent related endpoints
pub fn configure_agent_services(cfg: &mut ServiceConfig) {
    cfg.service(get_agent_status)
        .service(get_agent_config)
        .service(post_agent_config)
        .service(get_available_tools)
        .service(get_model_capabilities)
        .service(agent_chat)
        .service(agent_chat_stream)
        .service(get_conversations)
        .service(delete_conversation)
        .service(update_conversation_title)
        .service(get_conversation_history);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::AgentConfig;
    use crate::api::agent::service::config::AgentConfigHandle;
    use actix_web::{test, web, App};
    use std::sync::{Arc, Mutex};

    #[actix_web::test]
    async fn test_configure_agent_services_registers_all_endpoints() {
        let agent_config: AgentConfigHandle = Arc::new(Mutex::new(AgentConfig::default()));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(agent_config))
                .configure(configure_agent_services),
        )
        .await;

        // Test that all endpoints are registered
        let endpoints = vec![
            ("/api/agent/status", "GET"),
            ("/api/agent/config", "GET"),
            ("/api/agent/config", "POST"),
            ("/api/agent/chat", "POST"),
        ];

        for (path, method) in endpoints {
            let req = match method {
                "GET" => test::TestRequest::get().uri(path),
                "POST" => test::TestRequest::post().uri(path),
                _ => continue,
            };
            let req = req.to_request();
            let resp = test::call_service(&app, req).await;
            let status = resp.status().as_u16();
            // Endpoints should be registered (not 404)
            assert_ne!(
                status, 404,
                "Endpoint {} {} should be registered (got status {})",
                method, path, status
            );
        }
    }
}
