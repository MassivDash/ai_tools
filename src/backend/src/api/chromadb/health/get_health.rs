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
