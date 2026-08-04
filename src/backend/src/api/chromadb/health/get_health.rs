use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{ChromaDBHealthResponse, ChromaDBResponse};
use actix_web::{get, web, HttpResponse, Result as ActixResult};

#[get("/api/chromadb/health")]
pub async fn get_chromadb_health(chroma_address: web::Data<String>) -> ActixResult<HttpResponse> {
    println!(
        "🔍 Checking ChromaDB health at: {}",
        chroma_address.as_str()
    );

    match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(client) => match client.health_check().await {
            Ok(connected) => {
                let status = if connected { "healthy" } else { "unhealthy" };
                println!("✅ ChromaDB health check result: {}", status);
                Ok(HttpResponse::Ok().json(ChromaDBResponse {
                    success: true,
                    data: Some(ChromaDBHealthResponse {
                        status: status.to_string(),
                        version: "0.1.0".to_string(),
                        chromadb: crate::api::chromadb::types::ChromaDBStatus { connected },
                    }),
                    error: None,
                    message: None,
                }))
            }
            Err(e) => {
                println!("ChromaDB health check failed: {}", e);
                Ok(HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<
                    ChromaDBHealthResponse,
                > {
                    success: false,
                    data: Some(ChromaDBHealthResponse {
                        status: "unhealthy".to_string(),
                        version: "0.1.0".to_string(),
                        chromadb: crate::api::chromadb::types::ChromaDBStatus { connected: false },
                    }),
                    error: Some(e.to_string()),
                    message: None,
                }))
            }
        },
        Err(e) => {
            println!("Failed to create ChromaDB client: {}", e);
            Ok(HttpResponse::ServiceUnavailable().json(
                ChromaDBResponse::<ChromaDBHealthResponse> {
                    success: false,
                    data: Some(ChromaDBHealthResponse {
                        status: "unhealthy".to_string(),
                        version: "0.1.0".to_string(),
                        chromadb: crate::api::chromadb::types::ChromaDBStatus { connected: false },
                    }),
                    error: Some(e.to_string()),
                    message: None,
                },
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        lock_chroma_endpoint, MockChroma, MockChromaCollection, MockChromaConfig,
        UNPARSEABLE_CHROMA_ENDPOINT,
    };
    use actix_web::{test, App};

    async fn health_of(address: &str) -> (u16, ChromaDBResponse<ChromaDBHealthResponse>) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(address.to_string()))
                .service(get_chromadb_health),
        )
        .await;
        let resp = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/api/chromadb/health")
                .to_request(),
        )
        .await;
        let status = resp.status().as_u16();
        (status, test::read_body_json(resp).await)
    }

    #[actix_web::test]
    async fn test_health_is_reported_healthy_when_the_server_answers() {
        let chroma = MockChroma::start(MockChromaConfig::holding(vec![MockChromaCollection::new(
            "notes",
        )]))
        .await;

        let guard = lock_chroma_endpoint();
        let (status, body) = health_of(&chroma.base_url).await;
        drop(guard);

        assert_eq!(status, 200);
        assert!(body.success);
        assert!(body.error.is_none());
        let health = body.data.unwrap();
        assert_eq!(health.status, "healthy");
        assert_eq!(health.version, "0.1.0");
        assert!(health.chromadb.connected);

        chroma.stop().await;
    }

    /// A reachable-but-unhappy server is still a 200: `health_check` reports the
    /// failure as `connected: false` rather than as an error.
    #[actix_web::test]
    async fn test_health_is_reported_unhealthy_when_the_server_rejects_the_probe() {
        let chroma = MockChroma::start(MockChromaConfig {
            list_status: Some(400),
            ..MockChromaConfig::empty()
        })
        .await;

        let guard = lock_chroma_endpoint();
        let (status, body) = health_of(&chroma.base_url).await;
        drop(guard);

        assert_eq!(status, 200);
        assert!(body.success);
        let health = body.data.unwrap();
        assert_eq!(health.status, "unhealthy");
        assert!(!health.chromadb.connected);

        chroma.stop().await;
    }

    #[actix_web::test]
    async fn test_health_is_reported_unhealthy_when_nothing_is_listening() {
        let _guard = lock_chroma_endpoint();
        let (status, body) = health_of("http://127.0.0.1:1").await;

        assert_eq!(status, 200);
        let health = body.data.unwrap();
        assert_eq!(health.status, "unhealthy");
        assert!(!health.chromadb.connected);
    }

    #[actix_web::test]
    async fn test_health_reports_an_unusable_address_as_503() {
        let _guard = lock_chroma_endpoint();
        let (status, body) = health_of(UNPARSEABLE_CHROMA_ENDPOINT).await;

        assert_eq!(status, 503);
        assert!(!body.success);
        assert!(body
            .error
            .unwrap()
            .contains("Failed to create ChromaDB client"));
        // Even the failure carries a body, so the UI always has something to show.
        let health = body.data.unwrap();
        assert_eq!(health.status, "unhealthy");
        assert!(!health.chromadb.connected);
    }
}
