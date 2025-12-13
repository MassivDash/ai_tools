use actix_web::{post, web, HttpResponse, Result as ActixResult};
use serde::Deserialize;
use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{ChromaDBResponse, Collection};
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
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to create ChromaDB client: {}", e);
            return Ok(HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<Collection> {
                success: false,
                data: None,
                error: Some(e.to_string()),
                message: None,
            }));
        }
    };
    
    match client
        .create_collection(&req.name, req.metadata.clone())
        .await
    {
        Ok(collection) => Ok(HttpResponse::Ok().json(ChromaDBResponse {
            success: true,
            data: Some(collection),
            error: None,
            message: None,
        })),
        Err(e) => {
            println!("❌ Failed to create collection: {}", e);
            Ok(HttpResponse::InternalServerError().json(ChromaDBResponse::<Collection> {
                success: false,
                data: None,
                error: Some(e.to_string()),
                message: None,
            }))
        }
    }
}

