use actix_web::web;

use crate::api::games;

pub fn configure_games_services(cfg: &mut web::ServiceConfig) {
    cfg.configure(games::config);
}
