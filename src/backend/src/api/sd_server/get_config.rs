use crate::api::sd_server::types::SDConfigHandle;
use actix_web::{get, web, HttpResponse, Result as ActixResult};

#[get("/api/sd-server/config")]
pub async fn get_sd_config(config: web::Data<SDConfigHandle>) -> ActixResult<HttpResponse> {
    let config_guard = config.lock().unwrap();
    // Return the config directly since it now implements Serialize
    Ok(HttpResponse::Ok().json(&*config_guard))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sd_server::types::SDConfig;
    use actix_web::{test, App};
    use std::sync::{Arc, Mutex};

    #[actix_web::test]
    async fn test_get_sd_config_returns_the_defaults() {
        let config: SDConfigHandle = Arc::new(Mutex::new(SDConfig::default()));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .service(get_sd_config),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/sd-server/config")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: SDConfig = test::read_body_json(resp).await;
        assert_eq!(body.output_path, "./public");
        assert_eq!(body.models_path, "./sd_models");
        assert_eq!(body.width, 1024);
        assert_eq!(body.height, 1024);
        assert!(body.steps.is_none());
    }

    #[actix_web::test]
    async fn test_get_sd_config_reflects_mutations_made_through_the_handle() {
        let config: SDConfigHandle = Arc::new(Mutex::new(SDConfig::default()));
        {
            let mut guard = config.lock().unwrap();
            guard.prompt = "a robot".to_string();
            guard.steps = Some(30);
            guard.seed = Some(-1);
            guard.diffusion_model = "custom.gguf".to_string();
        }

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .service(get_sd_config),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/sd-server/config")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body: SDConfig = test::read_body_json(resp).await;
        assert_eq!(body.prompt, "a robot");
        assert_eq!(body.steps, Some(30));
        assert_eq!(body.seed, Some(-1));
        assert_eq!(body.diffusion_model, "custom.gguf");
    }
}
