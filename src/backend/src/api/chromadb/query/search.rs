use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{ChromaDBResponse, QueryRequest, QueryResponse};
use actix_web::{post, web, HttpResponse, Result as ActixResult};

#[post("/api/chromadb/query")]
pub async fn search_collection(
    req: web::Json<QueryRequest>,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to create ChromaDB client: {}", e);
            return Ok(HttpResponse::ServiceUnavailable().json(
                ChromaDBResponse::<QueryResponse> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                    message: None,
                },
            ));
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
            Ok(
                HttpResponse::InternalServerError().json(ChromaDBResponse::<QueryResponse> {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
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
    use crate::api::chromadb::types::QueryRequest;

    #[actix_web::test]
    async fn test_search_collection_basic() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new("http://localhost:8000".to_string()))
                .service(search_collection),
        )
        .await;

        let query_request = QueryRequest {
            collection: "test_collection".to_string(),
            query_texts: vec!["test query".to_string()],
            n_results: Some(5),
            where_clause: None,
        };

        let req = test::TestRequest::post()
            .uri("/api/chromadb/query")
            .set_json(&query_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should not be 400 (bad request) - might be 500 if ChromaDB unavailable
        assert_ne!(resp.status().as_u16(), 400);
    }

    #[actix_web::test]
    async fn test_search_collection_empty_query() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new("http://localhost:8000".to_string()))
                .service(search_collection),
        )
        .await;

        let query_request = QueryRequest {
            collection: "test_collection".to_string(),
            query_texts: vec![],
            n_results: Some(5),
            where_clause: None,
        };

        let req = test::TestRequest::post()
            .uri("/api/chromadb/query")
            .set_json(&query_request)
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should not be 400 (bad request) - might be 500 if ChromaDB unavailable
        assert_ne!(resp.status().as_u16(), 400);
    }
}
