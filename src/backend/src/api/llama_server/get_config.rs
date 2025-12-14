use actix_web::{get, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::api::llama_server::types::Config;

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigResponse {
    pub hf_model: String,
    pub ctx_size: u32,
    pub threads: Option<i32>,
    pub threads_batch: Option<i32>,
    pub predict: Option<i32>,
    pub batch_size: Option<u32>,
    pub ubatch_size: Option<u32>,
    pub flash_attn: Option<bool>,
    pub mlock: Option<bool>,
    pub no_mmap: Option<bool>,
    pub gpu_layers: Option<u32>,
    pub model: Option<String>,
}

#[get("/api/llama-server/config")]
pub async fn get_llama_config(config: web::Data<Arc<Mutex<Config>>>) -> ActixResult<HttpResponse> {
    let config_guard = config.lock().unwrap();
    Ok(HttpResponse::Ok().json(ConfigResponse {
        hf_model: config_guard.hf_model.clone(),
        ctx_size: config_guard.ctx_size,
        threads: config_guard.threads,
        threads_batch: config_guard.threads_batch,
        predict: config_guard.predict,
        batch_size: config_guard.batch_size,
        ubatch_size: config_guard.ubatch_size,
        flash_attn: config_guard.flash_attn,
        mlock: config_guard.mlock,
        no_mmap: config_guard.no_mmap,
        gpu_layers: config_guard.gpu_layers,
        model: config_guard.model.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};
    use crate::api::llama_server::types::Config;
    use std::sync::{Arc, Mutex};

    #[actix_web::test]
    async fn test_get_llama_config_default() {
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .service(get_llama_config),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/llama-server/config")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body: ConfigResponse = test::read_body_json(resp).await;
        assert_eq!(body.ctx_size, 10240);
        assert!(body.hf_model.contains("DeepSeek"));
    }

    #[actix_web::test]
    async fn test_get_llama_config_custom() {
        let config = Config {
            hf_model: "test/model".to_string(),
            ctx_size: 2048,
            threads: Some(4),
            threads_batch: Some(8),
            predict: Some(256),
            batch_size: Some(512),
            ubatch_size: Some(256),
            flash_attn: Some(true),
            mlock: Some(false),
            no_mmap: Some(true),
            gpu_layers: Some(10),
            model: Some("/path/to/model".to_string()),
        };
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(config));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config))
                .service(get_llama_config),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/llama-server/config")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body: ConfigResponse = test::read_body_json(resp).await;
        assert_eq!(body.hf_model, "test/model");
        assert_eq!(body.ctx_size, 2048);
        assert_eq!(body.threads, Some(4));
        assert_eq!(body.gpu_layers, Some(10));
        assert_eq!(body.model, Some("/path/to/model".to_string()));
    }
}
