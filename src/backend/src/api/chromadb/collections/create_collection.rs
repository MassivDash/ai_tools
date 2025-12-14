use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{ChromaDBResponse, Collection, DistanceMetric};
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub metadata: Option<HashMap<String, String>>,
    #[serde(default)]
    pub distance_metric: Option<DistanceMetric>,
}

#[post("/api/chromadb/collections")]
pub async fn create_collection(
    req: web::Json<CreateCollectionRequest>,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    println!(
        "📝 Creating collection request: name={}, metadata={:?}, distance_metric={:?}",
        req.name, req.metadata, req.distance_metric
    );

    // Validate collection name
    let collection_name = req.name.trim();
    if collection_name.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(ChromaDBResponse::<Collection> {
                success: false,
                data: None,
                error: Some("Collection name cannot be empty".to_string()),
                message: None,
            }),
        );
    }

    // Basic validation - ChromaDB will handle more specific validation
    // Just ensure it's not empty and not too long
    if collection_name.len() > 100 {
        return Ok(
            HttpResponse::BadRequest().json(ChromaDBResponse::<Collection> {
                success: false,
                data: None,
                error: Some("Collection name is too long (max 100 characters)".to_string()),
                message: None,
            }),
        );
    }

    // ChromaDB collection names cannot contain spaces or special characters
    // They must be valid identifiers (alphanumeric, underscores, hyphens)
    // Replace spaces with underscores and remove invalid characters
    let sanitized_name: String = collection_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_' // Replace spaces and other special chars with underscore
            }
        })
        .collect();

    // Ensure it doesn't start or end with underscore/hyphen
    let sanitized_name = sanitized_name.trim_matches(|c| c == '_' || c == '-');

    if sanitized_name.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(ChromaDBResponse::<Collection> {
                success: false,
                data: None,
                error: Some("Collection name contains only invalid characters".to_string()),
                message: None,
            }),
        );
    }

    // If name was sanitized, log it
    if sanitized_name != collection_name {
        println!(
            "⚠️ Collection name sanitized: '{}' -> '{}'",
            collection_name, sanitized_name
        );
    }

    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to create ChromaDB client: {}", e);
            return Ok(
                HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<Collection> {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to connect to ChromaDB: {}", e)),
                    message: None,
                }),
            );
        }
    };

    println!(
        "✅ ChromaDB client created, attempting to create collection: {} (sanitized: {})",
        collection_name, sanitized_name
    );

    // Merge distance_metric into metadata if provided
    // ChromaDB accepts distance metric via metadata key "hnsw:space" or "distance_metric"
    let mut metadata = req.metadata.clone().unwrap_or_default();
    if let Some(metric) = &req.distance_metric {
        let metric_str = match metric {
            DistanceMetric::Cosine => "cosine",
            DistanceMetric::L2 => "l2",
            DistanceMetric::Ip => "ip",
        };
        metadata.insert("hnsw:space".to_string(), metric_str.to_string());
        println!("🔧 Setting distance metric to: {}", metric_str);
    }

    match client
        .create_collection(sanitized_name, Some(metadata))
        .await
    {
        Ok(collection) => {
            println!("✅ Collection created successfully: {}", collection.name);
            Ok(HttpResponse::Ok().json(ChromaDBResponse {
                success: true,
                data: Some(collection),
                error: None,
                message: None,
            }))
        }
        Err(e) => {
            println!("❌ Failed to create collection '{}': {}", req.name, e);
            println!("   Error details: {:?}", e);
            Ok(
                HttpResponse::InternalServerError().json(ChromaDBResponse::<Collection> {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to create collection: {}", e)),
                    message: None,
                }),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};
    use std::collections::HashMap;

    #[actix_web::test]
    async fn test_create_collection_empty_name() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new("http://localhost:8000".to_string()))
                .service(create_collection),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/chromadb/collections")
            .set_json(&CreateCollectionRequest {
                name: "   ".to_string(), // Empty after trim
                metadata: None,
                distance_metric: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 400);
    }

    #[actix_web::test]
    async fn test_create_collection_name_too_long() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new("http://localhost:8000".to_string()))
                .service(create_collection),
        )
        .await;

        let long_name = "a".repeat(101); // 101 characters

        let req = test::TestRequest::post()
            .uri("/api/chromadb/collections")
            .set_json(&CreateCollectionRequest {
                name: long_name,
                metadata: None,
                distance_metric: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 400);
    }

    #[actix_web::test]
    async fn test_create_collection_name_sanitization() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new("http://localhost:8000".to_string()))
                .service(create_collection),
        )
        .await;

        // Test that names with spaces get sanitized
        let req = test::TestRequest::post()
            .uri("/api/chromadb/collections")
            .set_json(&CreateCollectionRequest {
                name: "test collection name".to_string(),
                metadata: None,
                distance_metric: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should not be 400 (bad request) - sanitization should handle it
        // It might fail with 500 if ChromaDB is not available, but that's OK
        assert_ne!(resp.status().as_u16(), 400);
    }

    #[actix_web::test]
    async fn test_create_collection_with_metadata() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new("http://localhost:8000".to_string()))
                .service(create_collection),
        )
        .await;

        let mut metadata = HashMap::new();
        metadata.insert("description".to_string(), "test collection".to_string());

        let req = test::TestRequest::post()
            .uri("/api/chromadb/collections")
            .set_json(&CreateCollectionRequest {
                name: "test_collection".to_string(),
                metadata: Some(metadata),
                distance_metric: None,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should not be 400 (bad request)
        assert_ne!(resp.status().as_u16(), 400);
    }
}
