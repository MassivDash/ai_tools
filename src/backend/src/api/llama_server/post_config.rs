use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;

use crate::api::llama_server::types::Config;

#[derive(Deserialize, Serialize, Debug)]
pub struct ConfigRequest {
    pub hf_model: Option<String>,
    pub ctx_size: Option<u32>,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ConfigResponse {
    pub success: bool,
    pub message: String,
}

#[post("/api/llama-server/config")]
pub async fn post_update_config(
    body: web::Json<ConfigRequest>,
    config: web::Data<Arc<Mutex<Config>>>,
) -> ActixResult<HttpResponse> {
    let mut config_guard = config.lock().unwrap();

    if let Some(hf_model) = &body.hf_model {
        if !hf_model.trim().is_empty() {
            config_guard.hf_model = hf_model.trim().to_string();
            println!("📝 Updated HF model to: {}", config_guard.hf_model);
        }
    }

    if let Some(ctx_size) = body.ctx_size {
        if ctx_size > 0 {
            config_guard.ctx_size = ctx_size;
            println!("📝 Updated context size to: {}", config_guard.ctx_size);
        }
    }

    if let Some(threads) = body.threads {
        config_guard.threads = Some(threads);
        println!("📝 Updated threads to: {:?}", config_guard.threads);
    }

    if let Some(threads_batch) = body.threads_batch {
        config_guard.threads_batch = Some(threads_batch);
        println!(
            "📝 Updated threads-batch to: {:?}",
            config_guard.threads_batch
        );
    }

    if let Some(predict) = body.predict {
        config_guard.predict = Some(predict);
        println!("📝 Updated predict to: {:?}", config_guard.predict);
    }

    if let Some(batch_size) = body.batch_size {
        if batch_size > 0 {
            config_guard.batch_size = Some(batch_size);
            println!("📝 Updated batch-size to: {:?}", config_guard.batch_size);
        }
    }

    if let Some(ubatch_size) = body.ubatch_size {
        if ubatch_size > 0 {
            config_guard.ubatch_size = Some(ubatch_size);
            println!("📝 Updated ubatch-size to: {:?}", config_guard.ubatch_size);
        }
    }

    if let Some(flash_attn) = body.flash_attn {
        config_guard.flash_attn = Some(flash_attn);
        println!("📝 Updated flash-attn to: {:?}", config_guard.flash_attn);
    }

    if let Some(mlock) = body.mlock {
        config_guard.mlock = Some(mlock);
        println!("📝 Updated mlock to: {:?}", config_guard.mlock);
    }

    if let Some(no_mmap) = body.no_mmap {
        config_guard.no_mmap = Some(no_mmap);
        println!("📝 Updated no-mmap to: {:?}", config_guard.no_mmap);
    }

    if let Some(gpu_layers) = body.gpu_layers {
        config_guard.gpu_layers = Some(gpu_layers);
        println!("📝 Updated gpu-layers to: {:?}", config_guard.gpu_layers);
    }

    if let Some(model) = &body.model {
        if !model.trim().is_empty() {
            config_guard.model = Some(model.trim().to_string());
            println!("📝 Updated model to: {:?}", config_guard.model);
        }
    }

    Ok(HttpResponse::Ok().json(ConfigResponse {
        success: true,
        message: "Configuration updated successfully".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::llama_server::types::Config;
    use actix_web::{test, web, App};
    use std::sync::{Arc, Mutex};

    #[actix_web::test]
    async fn test_post_update_config_hf_model() {
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .service(post_update_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/llama-server/config")
            .set_json(&ConfigRequest {
                hf_model: Some("test/model:Q4_K_M".to_string()),
                ctx_size: None,
                threads: None,
                threads_batch: None,
                predict: None,
                batch_size: None,
                ubatch_size: None,
                flash_attn: None,
                mlock: None,
                no_mmap: None,
                gpu_layers: None,
                model: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: ConfigResponse = test::read_body_json(resp).await;
        assert!(body.success);
        assert_eq!(body.message, "Configuration updated successfully");

        // Verify config was updated
        let config_guard = config.lock().unwrap();
        assert_eq!(config_guard.hf_model, "test/model:Q4_K_M");
    }

    #[actix_web::test]
    async fn test_post_update_config_multiple_fields() {
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .service(post_update_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/llama-server/config")
            .set_json(&ConfigRequest {
                hf_model: Some("test/model".to_string()),
                ctx_size: Some(2048),
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
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: ConfigResponse = test::read_body_json(resp).await;
        assert!(body.success);

        // Verify all config fields were updated
        let config_guard = config.lock().unwrap();
        assert_eq!(config_guard.hf_model, "test/model");
        assert_eq!(config_guard.ctx_size, 2048);
        assert_eq!(config_guard.threads, Some(4));
        assert_eq!(config_guard.gpu_layers, Some(10));
    }

    #[actix_web::test]
    async fn test_post_update_config_empty_hf_model_ignored() {
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));
        let original_model = config.lock().unwrap().hf_model.clone();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .service(post_update_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/llama-server/config")
            .set_json(&ConfigRequest {
                hf_model: Some("   ".to_string()), // Empty after trim
                ctx_size: None,
                threads: None,
                threads_batch: None,
                predict: None,
                batch_size: None,
                ubatch_size: None,
                flash_attn: None,
                mlock: None,
                no_mmap: None,
                gpu_layers: None,
                model: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Verify config was not updated (empty string ignored)
        let config_guard = config.lock().unwrap();
        assert_eq!(config_guard.hf_model, original_model);
    }

    #[actix_web::test]
    async fn test_post_update_config_invalid_ctx_size_ignored() {
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));
        let original_ctx_size = config.lock().unwrap().ctx_size;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .service(post_update_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/llama-server/config")
            .set_json(&ConfigRequest {
                hf_model: None,
                ctx_size: Some(0), // Invalid (must be > 0)
                threads: None,
                threads_batch: None,
                predict: None,
                batch_size: None,
                ubatch_size: None,
                flash_attn: None,
                mlock: None,
                no_mmap: None,
                gpu_layers: None,
                model: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Verify ctx_size was not updated (0 is invalid)
        let config_guard = config.lock().unwrap();
        assert_eq!(config_guard.ctx_size, original_ctx_size);
    }
}
