pub mod service;
pub mod types;
pub mod websocket;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(service::game_chat_stream);
    cfg.service(websocket::game_ws_route);
}
