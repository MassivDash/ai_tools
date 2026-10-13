use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::Mutex;

use crate::api::default_configs::{DefaultConfigsStorage, LlamaDefaultConfig};
use crate::api::llama_server::types::Config;

/// Numeric options use `Option<Option<T>>` so the request can tell apart
/// a missing key (keep the current value) from `null` (clear it).
#[derive(Deserialize, Serialize, Debug)]
pub struct ConfigRequest {
    pub hf_model: Option<String>,
    pub ctx_size: Option<u32>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub threads: Option<Option<i32>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub threads_batch: Option<Option<i32>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub predict: Option<Option<i32>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub batch_size: Option<Option<u32>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub ubatch_size: Option<Option<u32>>,
    pub flash_attn: Option<bool>,
    pub mlock: Option<bool>,
    pub no_mmap: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub gpu_layers: Option<Option<u32>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub n_cpu_moe: Option<Option<u32>>,
    pub model: Option<String>,
}

/// Marks a key as present, even when its value is `null`.
fn present<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// Applies a keep/clear/set update to an optional numeric field.
fn apply<T: Copy + std::fmt::Debug>(target: &mut Option<T>, update: Option<Option<T>>, name: &str) {
    if let Some(value) = update {
        *target = value;
        println!("📝 Updated {} to: {:?}", name, target);
    }
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
    default_configs: web::Data<Arc<DefaultConfigsStorage>>,
) -> ActixResult<HttpResponse> {
    // Update all config fields (scope mutex guard)
    {
        let mut config_guard = config.lock().unwrap();

        if let Some(hf_model) = &body.hf_model {
            if !hf_model.trim().is_empty() {
                config_guard.hf_model = hf_model.trim().to_string();
                println!("📝 Updated HF model to: {}", config_guard.hf_model);
            } else {
                config_guard.hf_model = String::new();
                println!("📝 Cleared HF model");
            }
        }

        if let Some(ctx_size) = body.ctx_size {
            config_guard.ctx_size = ctx_size;
            println!("📝 Updated context size to: {}", config_guard.ctx_size);
        }

        apply(&mut config_guard.threads, body.threads, "threads");
        apply(
            &mut config_guard.threads_batch,
            body.threads_batch,
            "threads-batch",
        );
        apply(&mut config_guard.predict, body.predict, "predict");
        // A batch size of 0 is invalid for llama.cpp, so it is ignored.
        let non_zero = |v: &Option<u32>| *v != Some(0);
        apply(
            &mut config_guard.batch_size,
            body.batch_size.filter(non_zero),
            "batch-size",
        );
        apply(
            &mut config_guard.ubatch_size,
            body.ubatch_size.filter(non_zero),
            "ubatch-size",
        );

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

        apply(&mut config_guard.gpu_layers, body.gpu_layers, "gpu-layers");
        apply(&mut config_guard.n_cpu_moe, body.n_cpu_moe, "n-cpu-moe");

        if let Some(model) = &body.model {
            if model.trim().is_empty() {
                config_guard.model = None;
                println!("📝 Cleared model path");
            } else {
                config_guard.model = Some(model.trim().to_string());
                println!("📝 Updated model to: {:?}", config_guard.model);
            }
        }
    }

    // Save full snapshot as default config in SQLite
    let snapshot = {
        let config_guard = config.lock().unwrap();
        LlamaDefaultConfig {
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
            n_cpu_moe: config_guard.n_cpu_moe,
            model: config_guard.model.clone(),
        }
    };

    if let Err(e) = default_configs.set_llama_default(&snapshot).await {
        println!("⚠️  Failed to save llama default config: {}", e);
    } else {
        println!("✅ Saved llama default config to SQLite");
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

    // Helper to create a test default_configs storage with in-memory database
    async fn create_test_default_configs() -> Arc<DefaultConfigsStorage> {
        let storage = DefaultConfigsStorage::new(":memory:")
            .await
            .expect("Failed to create test default configs storage");
        Arc::new(storage)
    }

    #[actix_web::test]
    async fn test_post_update_config_null_clears_numeric_and_missing_keeps_it() {
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config {
            threads: Some(8),
            threads_batch: Some(4),
            predict: Some(128),
            batch_size: Some(512),
            ubatch_size: Some(256),
            gpu_layers: Some(99),
            n_cpu_moe: Some(32),
            ..Config::default()
        }));
        let default_configs = create_test_default_configs().await;
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .app_data(web::Data::new(default_configs))
                .service(post_update_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/llama-server/config")
            .set_json(serde_json::json!({
                "threads": null,
                "threads_batch": null,
                "predict": null,
                "batch_size": null,
                "ubatch_size": null,
                "n_cpu_moe": null
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let guard = config.lock().unwrap();
        assert_eq!(guard.threads, None);
        assert_eq!(guard.threads_batch, None);
        assert_eq!(guard.predict, None);
        assert_eq!(guard.batch_size, None);
        assert_eq!(guard.ubatch_size, None);
        assert_eq!(guard.n_cpu_moe, None);
        // gpu_layers was not in the request, so it is kept.
        assert_eq!(guard.gpu_layers, Some(99));
    }

    #[actix_web::test]
    async fn test_post_update_config_hf_model() {
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));
        let default_configs = create_test_default_configs().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .app_data(web::Data::new(default_configs))
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
                n_cpu_moe: None,
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
        let default_configs = create_test_default_configs().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .app_data(web::Data::new(default_configs))
                .service(post_update_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/llama-server/config")
            .set_json(&ConfigRequest {
                hf_model: Some("test/model".to_string()),
                ctx_size: Some(2048),
                threads: Some(Some(4)),
                threads_batch: Some(Some(8)),
                predict: Some(Some(256)),
                batch_size: Some(Some(512)),
                ubatch_size: Some(Some(256)),
                flash_attn: Some(true),
                mlock: Some(false),
                no_mmap: Some(true),
                gpu_layers: Some(Some(10)),
                n_cpu_moe: Some(Some(32)),
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
        assert_eq!(config_guard.n_cpu_moe, Some(32));
    }

    #[actix_web::test]
    async fn test_post_update_config_empty_hf_model_cleared() {
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));
        let default_configs = create_test_default_configs().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .app_data(web::Data::new(default_configs))
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
                n_cpu_moe: None,
                model: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Verify config was updated to empty string
        let config_guard = config.lock().unwrap();
        assert_eq!(config_guard.hf_model, "");
    }

    #[actix_web::test]
    async fn test_post_update_config_ctx_size_zero_allowed() {
        let config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));
        let default_configs = create_test_default_configs().await;

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config.clone()))
                .app_data(web::Data::new(default_configs))
                .service(post_update_config),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/llama-server/config")
            .set_json(&ConfigRequest {
                hf_model: None,
                ctx_size: Some(0), // Valid (0 means auto)
                threads: None,
                threads_batch: None,
                predict: None,
                batch_size: None,
                ubatch_size: None,
                flash_attn: None,
                mlock: None,
                no_mmap: None,
                gpu_layers: None,
                n_cpu_moe: None,
                model: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        // Verify ctx_size was updated to 0
        let config_guard = config.lock().unwrap();
        assert_eq!(config_guard.ctx_size, 0);
    }
}
