use actix_web::web::ServiceConfig;

use crate::api::llama_server::get_config::get_llama_config;
use crate::api::llama_server::get_logs::get_llama_logs;
use crate::api::llama_server::get_models::get_llama_models;
use crate::api::llama_server::get_status::get_llama_server_status;
use crate::api::llama_server::post_config::post_update_config;
use crate::api::llama_server::post_start::post_start_llama_server;
use crate::api::llama_server::post_stop::post_stop_llama_server;

/// Configures all llama-server related endpoints
pub fn configure_llama_server_services(cfg: &mut ServiceConfig) {
    cfg.service(get_llama_server_status)
        .service(get_llama_models)
        .service(get_llama_config)
        .service(get_llama_logs)
        .service(post_start_llama_server)
        .service(post_stop_llama_server)
        .service(post_update_config);
}

