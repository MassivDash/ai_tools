use actix_web::{delete, web, HttpResponse, Result as ActixResult};
use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::ChromaDBResponse;

#[delete("/api/chromadb/collections/{name}")]
pub async fn delete_collection(
    path: web::Path<String>,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to create ChromaDB client: {}", e);
            return Ok(HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<()> {
                success: false,
                data: None,
                error: Some(e.to_string()),
                message: None,
            }));
        }
    };
    
    match client.delete_collection(&name).await {
        Ok(_) => Ok(HttpResponse::Ok().json(ChromaDBResponse::<()> {
            success: true,
            data: None,
            error: None,
            message: Some(format!("Collection {} deleted successfully", name)),
        })),
        Err(e) => {
            println!("❌ Failed to delete collection: {}", e);
            Ok(HttpResponse::InternalServerError().json(ChromaDBResponse::<()> {
                success: false,
                data: None,
                error: Some(e.to_string()),
                message: None,
            }))
        }
    }
}

