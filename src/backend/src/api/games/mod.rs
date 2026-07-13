pub mod one_of_ten;
pub mod service;
pub mod types;
pub mod websocket;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(service::game_chat_stream);
    cfg.service(websocket::game_ws_route);
    cfg.service(one_of_ten::one_of_ten_ws_route);
}
