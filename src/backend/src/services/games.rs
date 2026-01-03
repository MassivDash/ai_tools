use crate::api::games;
use actix_web::web;

pub fn configure_games_services(cfg: &mut web::ServiceConfig) {
    cfg.configure(games::config);
}
