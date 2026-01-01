use crate::api::sd_server::delete_image::delete_sd_image;
use crate::api::sd_server::get_config::get_sd_config;
use crate::api::sd_server::get_images::get_sd_images;
use crate::api::sd_server::post_config::post_update_sd_config;
use crate::api::sd_server::post_start::post_start_sd_server;
use actix_web::web;

pub fn configure_sd_server_services(cfg: &mut web::ServiceConfig) {
    cfg.service(post_start_sd_server)
        .service(post_update_sd_config)
        .service(get_sd_images)
        .service(delete_sd_image)
        .service(get_sd_config);
}
