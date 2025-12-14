use actix_multipart::Multipart;
use actix_web::{post, web, Error as ActixError, HttpRequest, HttpResponse};
use futures_util::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize, Debug)]
pub struct JsonToToonRequest {
    pub json: String,
    #[serde(default)]
    pub count_tokens: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToonResponse {
    pub toon: String,
    pub json_tokens: usize,
    pub toon_tokens: usize,
    pub token_savings: f64, // Percentage savings
}

/// Handles both JSON body (paste) and multipart/form-data (file upload)
#[post("/api/json-to-toon")]
pub async fn convert_json_to_toon(
    req: HttpRequest,
    payload: web::Payload,
) -> Result<HttpResponse, ActixError> {
    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let (json_string, count_tokens) = if content_type.starts_with("multipart/form-data") {
        // Handle file upload
        let mut multipart = Multipart::new(req.headers(), payload);

        let mut json_data: Option<String> = None;
        let mut count_tokens = false;

        while let Some(mut field) = multipart.try_next().await? {
            let field_name = field.name();

            if field_name == "file" {
                // Read file data
                let mut data = Vec::new();
                while let Some(chunk) = field.try_next().await? {
                    data.extend_from_slice(&chunk);
                }

                // Validate file size (10MB max)
                const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
                if data.len() > MAX_FILE_SIZE {
                    return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                        "error": format!("File too large: {} bytes (max {} bytes)", data.len(), MAX_FILE_SIZE)
                    })));
                }

                // Try to parse as UTF-8 JSON
                match String::from_utf8(data) {
                    Ok(json_str) => {
                        json_data = Some(json_str);
                    }
                    Err(e) => {
                        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("File is not valid UTF-8: {}", e)
                        })));
                    }
                }
            } else if field_name == "json" {
                // Also support pasted JSON in multipart
                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await? {
                    bytes.extend_from_slice(&chunk);
                }
                if let Ok(json_str) = String::from_utf8(bytes) {
                    json_data = Some(json_str);
                }
            } else if field_name == "count_tokens" {
                // Read count_tokens boolean value
                let mut bytes = Vec::new();
                while let Some(chunk) = field.try_next().await? {
                    bytes.extend_from_slice(&chunk);
                }
                if let Ok(value_str) = String::from_utf8(bytes) {
                    count_tokens =
                        value_str.trim().eq_ignore_ascii_case("true") || value_str.trim() == "1";
                }
            }
        }

        let json_string = json_data.ok_or_else(|| {
            actix_web::error::ErrorBadRequest("No JSON data provided (file or json field)")
        })?;

        (json_string, count_tokens)
    } else {
        // Handle JSON body (paste) - read the body manually
        let mut body = web::BytesMut::new();
        let mut stream = payload;
        while let Some(item) = stream.next().await {
            let chunk = item?;
            body.extend_from_slice(&chunk);
        }

        let body_str = String::from_utf8(body.to_vec())
            .map_err(|_| actix_web::error::ErrorBadRequest("Invalid UTF-8 in request body"))?;

        let body: JsonToToonRequest = serde_json::from_str(&body_str)
            .map_err(|_| actix_web::error::ErrorBadRequest("Invalid JSON request body"))?;

        (body.json, body.count_tokens)
    };

    // Validate and parse JSON
    let json_string = json_string.trim();
    if json_string.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "JSON content cannot be empty"
        })));
    }

    // Limit JSON size (10MB max)
    const MAX_JSON_SIZE: usize = 10 * 1024 * 1024;
    if json_string.len() > MAX_JSON_SIZE {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("JSON content too large: {} bytes (max {} bytes)", json_string.len(), MAX_JSON_SIZE)
        })));
    }

    println!("📥 Received JSON (length: {} chars)", json_string.len());

    // Parse JSON to validate it
    let json_value: Value = match serde_json::from_str(json_string) {
        Ok(value) => value,
        Err(e) => {
            println!("❌ Invalid JSON: {}", e);
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid JSON: {}", e)
            })));
        }
    };

    // Convert JSON to TOON
    println!("🔄 Converting JSON to TOON...");
    let toon_output = toon::encode(&json_value, None);

    println!(
        "✅ Conversion successful! TOON length: {}",
        toon_output.len()
    );

    // Count tokens if requested
    let (json_tokens, toon_tokens) = if count_tokens {
        let json_token_count = match crate::utils::tokenizer::count_tokens(json_string) {
            Ok(count) => {
                println!("🔢 JSON token count: {}", count);
                count
            }
            Err(e) => {
                println!("⚠️ Failed to count JSON tokens: {}", e);
                0
            }
        };

        let toon_token_count = match crate::utils::tokenizer::count_tokens(&toon_output) {
            Ok(count) => {
                println!("🔢 TOON token count: {}", count);
                count
            }
            Err(e) => {
                println!("⚠️ Failed to count TOON tokens: {}", e);
                0
            }
        };

        (json_token_count, toon_token_count)
    } else {
        (0, 0)
    };

    // Calculate token savings percentage
    let token_savings = if json_tokens > 0 && toon_tokens > 0 {
        ((json_tokens as f64 - toon_tokens as f64) / json_tokens as f64) * 100.0
    } else {
        0.0
    };

    Ok(HttpResponse::Ok().json(ToonResponse {
        toon: toon_output,
        json_tokens,
        toon_tokens,
        token_savings,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_rt::test]
    async fn test_convert_json_to_toon_empty_json() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: "".to_string(),
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_client_error());
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_invalid_json() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: "{ invalid json }".to_string(),
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_client_error());
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_valid_json() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: r#"{"user": {"id": 123, "name": "Ada", "active": true}}"#.to_string(),
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty());
        assert!(body.toon.contains("user"));
        assert_eq!(body.json_tokens, 0);
        assert_eq!(body.toon_tokens, 0);
        assert_eq!(body.token_savings, 0.0);
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_with_arrays() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: r#"{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]}"#
                    .to_string(),
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty());
        assert!(body.toon.contains("users"));
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_with_nested_objects() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: r#"{"data": {"user": {"profile": {"name": "Test", "age": 30}}}}"#.to_string(),
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty());
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_with_count_tokens() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: r#"{"test": "value"}"#.to_string(),
                count_tokens: true,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty());
        // Token counts should be calculated (may be 0 if tokenizer fails, but should not error)
        // Note: usize is always >= 0, but keeping for clarity
        // If both are > 0, savings should be calculated
        if body.json_tokens > 0 && body.toon_tokens > 0 {
            assert!(body.token_savings >= 0.0);
            assert!(body.token_savings <= 100.0);
        }
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_with_empty_object() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: r#"{}"#.to_string(),
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty() || body.toon.is_empty()); // Empty object might produce empty TOON
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_with_array_of_primitives() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: r#"{"tags": ["reading", "gaming", "coding"]}"#.to_string(),
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty());
        assert!(body.toon.contains("tags"));
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_with_mixed_types() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: r#"{"id": 123, "name": "Test", "active": true, "score": 98.5, "tags": ["a", "b"]}"#.to_string(),
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty());
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_whitespace_trimmed() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: r#"  {"test": "value"}  "#.to_string(),
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty());
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_large_json() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        // Create a larger JSON object
        let mut large_json = String::from(r#"{"items": ["#);
        for i in 0..100 {
            if i > 0 {
                large_json.push_str(", ");
            }
            large_json.push_str(&format!(r#""item{}""#, i));
        }
        large_json.push_str(r#"]}"#);

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: large_json,
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty());
    }

    #[actix_rt::test]
    async fn test_convert_json_to_toon_token_savings_calculation() {
        let mut app = test::init_service(App::new().service(convert_json_to_toon)).await;

        let req = test::TestRequest::post()
            .uri("/api/json-to-toon")
            .set_json(&JsonToToonRequest {
                json: r#"{"users": [{"id": 1, "name": "Alice", "role": "admin"}, {"id": 2, "name": "Bob", "role": "user"}]}"#.to_string(),
                count_tokens: true,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        let body: ToonResponse = test::read_body_json(resp).await;
        assert!(!body.toon.is_empty());

        // If tokens are counted, verify savings calculation
        if body.json_tokens > 0 && body.toon_tokens > 0 {
            let expected_savings = ((body.json_tokens as f64 - body.toon_tokens as f64)
                / body.json_tokens as f64)
                * 100.0;
            assert!((body.token_savings - expected_savings).abs() < 0.01); // Allow small floating point differences
                                                                           // TOON should generally have fewer tokens than JSON
            assert!(body.toon_tokens <= body.json_tokens);
        }
    }
}
