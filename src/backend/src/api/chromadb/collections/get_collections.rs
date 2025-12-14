use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::ChromaDBResponse;
use actix_web::{get, web, HttpResponse, Result as ActixResult};

#[get("/api/chromadb/collections")]
pub async fn get_collections(chroma_address: web::Data<String>) -> ActixResult<HttpResponse> {
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to create ChromaDB client: {}", e);
            return Ok(
                HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<Vec<()>> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                }),
            );
        }
    };

    match client.list_collections().await {
        Ok(collections) => Ok(HttpResponse::Ok().json(ChromaDBResponse {
            success: true,
            data: Some(collections),
            error: None,
            message: None,
        })),
        Err(e) => {
            println!("❌ Failed to list collections: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ChromaDBResponse::<Vec<()>> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                }),
            )
        }
    }
}
