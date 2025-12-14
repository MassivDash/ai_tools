use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{ChromaDBResponse, Collection};
use actix_web::{get, web, HttpResponse, Result as ActixResult};

#[get("/api/chromadb/collections/{name}")]
pub async fn get_collection(
    path: web::Path<String>,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    let name = path.into_inner();
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to create ChromaDB client: {}", e);
            return Ok(
                HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<Collection> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                }),
            );
        }
    };

    match client.get_collection(&name).await {
        Ok(collection) => Ok(HttpResponse::Ok().json(ChromaDBResponse {
            success: true,
            data: Some(collection),
            error: None,
            message: None,
        })),
        Err(e) => {
            println!("❌ Failed to get collection: {}", e);
            Ok(
                HttpResponse::NotFound().json(ChromaDBResponse::<Collection> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                }),
            )
        }
    }
}
