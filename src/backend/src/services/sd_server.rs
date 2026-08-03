use crate::api::sd_server::delete_image::delete_sd_image;
use crate::api::sd_server::get_config::get_sd_config;
use crate::api::sd_server::get_images::get_sd_images;
use crate::api::sd_server::post_config::post_update_sd_config;
use crate::api::sd_server::post_start::post_start_sd_server;
use actix_web::web;

use crate::api::sd_server::model_sets_handlers::{
    create_model_set, delete_model_set, list_model_sets, update_model_set,
};

pub fn configure_sd_server_services(cfg: &mut web::ServiceConfig) {
    cfg.service(post_start_sd_server)
        .service(post_update_sd_config)
        .service(get_sd_images)
        .service(delete_sd_image)
        .service(get_sd_config)
        .service(list_model_sets)
        .service(create_model_set)
        .service(update_model_set)
        .service(delete_model_set);
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    /// Registration is checked by confirming each route resolves to *something*
    /// other than a 404. Every handler here requires app data that is deliberately
    /// not provided, so extraction fails with a 500 before any handler body runs -
    /// in particular before `/start` could spawn the stable-diffusion process.
    #[actix_web::test]
    async fn test_configure_sd_server_services_registers_every_route() {
        let app = test::init_service(App::new().configure(configure_sd_server_services)).await;

        let routes = [
            ("POST", "/api/sd-server/start"),
            ("GET", "/api/sd-server/config"),
            ("POST", "/api/sd-server/config"),
            ("GET", "/api/sd-server/images"),
            ("DELETE", "/api/sd-server/image/some.png"),
            ("GET", "/api/sd-server/model-sets"),
            ("POST", "/api/sd-server/model-sets"),
            ("PUT", "/api/sd-server/model-sets/1"),
            ("DELETE", "/api/sd-server/model-sets/1"),
        ];

        for (method, path) in routes {
            let req = match method {
                "GET" => test::TestRequest::get(),
                "POST" => test::TestRequest::post(),
                "PUT" => test::TestRequest::put(),
                _ => test::TestRequest::delete(),
            }
            .uri(path)
            .to_request();

            let resp = test::call_service(&app, req).await;
            assert_ne!(
                resp.status().as_u16(),
                404,
                "{} {} should be registered",
                method,
                path
            );
        }
    }

    #[actix_web::test]
    async fn test_unregistered_sd_server_paths_are_not_found() {
        let app = test::init_service(App::new().configure(configure_sd_server_services)).await;

        let req = test::TestRequest::get()
            .uri("/api/sd-server/nope")
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status().as_u16(), 404);
    }
}
