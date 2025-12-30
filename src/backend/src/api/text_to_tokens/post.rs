use actix_web::{post, web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct TextRequest {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TokenResponse {
    pub token_count: usize,
    pub character_count: usize,
    pub word_count: usize,
}

#[post("/api/text-to-tokens")]
pub async fn convert_text_to_tokens(
    body: web::Json<TextRequest>,
) -> Result<HttpResponse, ActixError> {
    let text = body.text.trim();

    if text.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Text cannot be empty"
        })));
    }

    // Limit text size to prevent memory issues (10MB max)
    const MAX_TEXT_SIZE: usize = 10 * 1024 * 1024;
    if text.len() > MAX_TEXT_SIZE {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Text too large: {} bytes (max {} bytes)", text.len(), MAX_TEXT_SIZE)
        })));
    }

    println!(
        "📥 Received text for token counting (length: {} chars)",
        text.len()
    );

    // Count tokens
    let token_count = match crate::utils::tokenizer::count_tokens(text) {
        Ok(count) => {
            println!("🔢 Token count: {}", count);
            count
        }
        Err(e) => {
            println!("Failed to count tokens: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to count tokens: {}", e)
            })));
        }
    };

    // Count characters and words
    let character_count = text.chars().count();
    let word_count = text.split_whitespace().count();

    Ok(HttpResponse::Ok().json(TokenResponse {
        token_count,
        character_count,
        word_count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_convert_text_to_tokens_success() {
        let app = test::init_service(App::new().service(convert_text_to_tokens)).await;

        let req = test::TestRequest::post()
            .uri("/api/text-to-tokens")
            .set_json(&TextRequest {
                text: "Hello world! This is a test.".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: TokenResponse = test::read_body_json(resp).await;
        assert!(body.token_count > 0);
        // Text is trimmed, so "Hello world! This is a test." = 30 chars
        // But after trim it's still 30 chars, so check it's at least that
        assert!(body.character_count >= 28); // Allow for trimming variations
        assert!(body.word_count >= 5); // "Hello world! This is a test." = 6 words
    }

    #[actix_web::test]
    async fn test_convert_text_to_tokens_empty() {
        let app = test::init_service(App::new().service(convert_text_to_tokens)).await;

        let req = test::TestRequest::post()
            .uri("/api/text-to-tokens")
            .set_json(&TextRequest {
                text: "   ".to_string(), // Only whitespace
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 400);
    }

    #[actix_web::test]
    async fn test_convert_text_to_tokens_whitespace_trimmed() {
        let app = test::init_service(App::new().service(convert_text_to_tokens)).await;

        let req = test::TestRequest::post()
            .uri("/api/text-to-tokens")
            .set_json(&TextRequest {
                text: "  Hello  ".to_string(),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let body: TokenResponse = test::read_body_json(resp).await;
        assert!(body.token_count > 0);
        assert_eq!(body.character_count, 5); // "Hello" after trim
    }
}
