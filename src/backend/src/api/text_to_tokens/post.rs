use actix_web::{post, web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct TextRequest {
    pub text: String,
}

#[derive(Serialize, Debug)]
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

    println!("📥 Received text for token counting (length: {} chars)", text.len());

    // Count tokens
    let token_count = match crate::utils::tokenizer::count_tokens(text) {
        Ok(count) => {
            println!("🔢 Token count: {}", count);
            count
        }
        Err(e) => {
            println!("❌ Failed to count tokens: {}", e);
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

