use actix_web::{post, web, HttpResponse, Result as ActixResult};
use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{ChromaDBResponse, QueryRequest, QueryResponse};

#[post("/api/chromadb/query")]
pub async fn search_collection(
    req: web::Json<QueryRequest>,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to create ChromaDB client: {}", e);
            return Ok(HttpResponse::ServiceUnavailable().json(ChromaDBResponse::<QueryResponse> {
                success: false,
                data: None,
                error: Some(e.to_string()),
                message: None,
            }));
        }
    };
    
    match client.query(req.into_inner()).await {
        Ok(results) => Ok(HttpResponse::Ok().json(ChromaDBResponse {
            success: true,
            data: Some(results),
            error: None,
            message: None,
        })),
        Err(e) => {
            println!("❌ Failed to query collection: {}", e);
            Ok(HttpResponse::InternalServerError().json(ChromaDBResponse::<QueryResponse> {
                success: false,
                data: None,
                error: Some(e.to_string()),
                message: None,
            }))
        }
    }
}

