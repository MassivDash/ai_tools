use actix_web::web::ServiceConfig;

use crate::api::agent::tool_groups::{
    create_tool_group, delete_tool_group, get_tool_groups, update_tool_group,
};

/// Configures all tool groups related endpoints
pub fn configure_tool_groups_services(cfg: &mut ServiceConfig) {
    cfg.service(get_tool_groups)
        .service(create_tool_group)
        .service(update_tool_group)
        .service(delete_tool_group);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_configure_tool_groups_services_registers_all_endpoints() {
        let app = test::init_service(App::new().configure(configure_tool_groups_services)).await;

        let endpoints = vec![
            ("/api/agent/tool-groups", "GET"),
            ("/api/agent/tool-groups", "POST"),
            ("/api/agent/tool-groups/1", "PUT"),
            ("/api/agent/tool-groups/1", "DELETE"),
        ];

        for (path, method) in endpoints {
            let req = match method {
                "GET" => test::TestRequest::get().uri(path),
                "POST" => test::TestRequest::post().uri(path),
                "PUT" => test::TestRequest::put().uri(path),
                "DELETE" => test::TestRequest::delete().uri(path),
                _ => continue,
            };
            let req = req.to_request();
            let resp = test::call_service(&app, req).await;
            assert_ne!(
                resp.status().as_u16(),
                404,
                "Endpoint {} {} should be registered",
                method,
                path
            );
        }
    }
}
