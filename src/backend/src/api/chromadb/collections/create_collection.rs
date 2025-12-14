use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{ChromaDBResponse, Collection};
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub metadata: Option<HashMap<String, String>>,
}

#[post("/api/chromadb/collections")]
pub async fn create_collection(
    req: web::Json<CreateCollectionRequest>,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    println!(
        "📝 Creating collection request: name={}, metadata={:?}",
        req.name, req.metadata
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

    match client
        .create_collection(sanitized_name, req.metadata.clone())
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
