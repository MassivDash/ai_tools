use actix_multipart::Multipart;
use actix_web::{post, Error as ActixError, HttpResponse};
use futures_util::TryStreamExt;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct MarkdownResponse {
    pub markdown: String,
    pub filename: String,
    pub token_count: usize,
}

#[post("/api/pdf-to-markdown")]
pub async fn convert_pdf_to_markdown(mut payload: Multipart) -> Result<HttpResponse, ActixError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut count_tokens = false;

    // Parse multipart form data
    while let Some(mut field) = payload.try_next().await? {
        let field_name = field.name();

        if field_name == "file" {
            // Get filename from content disposition
            let content_disposition = field.content_disposition();
            if let Some(name) = content_disposition.get_filename() {
                filename = Some(name.to_string());
            }

            // Read file data
            let mut data = Vec::new();
            while let Some(chunk) = field.try_next().await? {
                data.extend_from_slice(&chunk);
            }
            file_data = Some(data);
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

    // Validate that we have a file
    let file_data = match file_data {
        Some(data) => {
            if data.is_empty() {
                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "No file data received"
                })));
            }
            data
        }
        None => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "No file provided"
            })));
        }
    };

    let filename = filename.unwrap_or_else(|| "document.pdf".to_string());

    // Validate file is PDF
    if !filename.to_lowercase().ends_with(".pdf") {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "File must be a PDF"
        })));
    }

    println!(
        "📥 Received PDF file: {} (size: {} bytes)",
        filename,
        file_data.len()
    );

    // Limit file size to prevent memory issues (50MB max)
    const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
    if file_data.len() > MAX_FILE_SIZE {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("File too large: {} bytes (max {} bytes)", file_data.len(), MAX_FILE_SIZE)
        })));
    }

    // Extract text from PDF
    println!("🔄 Extracting text from PDF...");
    let text = match extract_text_from_pdf(&file_data) {
        Ok(text) => {
            if text.trim().is_empty() {
                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "PDF appears to be empty or contains no extractable text"
                })));
            }
            println!("✅ Extracted {} characters from PDF", text.len());
            text
        }
        Err(e) => {
            println!("❌ Failed to extract text from PDF: {}", e);
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to extract text from PDF: {}", e)
            })));
        }
    };

    // Convert text to markdown
    // For now, we'll format the plain text as markdown
    // In the future, we could add more sophisticated formatting
    let markdown = format_text_as_markdown(&text);

    // Count tokens in the markdown only if requested (can be slow for large documents)
    let token_count = if count_tokens {
        match crate::utils::tokenizer::count_tokens(&markdown) {
            Ok(count) => {
                println!("🔢 Token count: {}", count);
                count
            }
            Err(e) => {
                println!("⚠️ Failed to count tokens: {}", e);
                0 // Return 0 if token counting fails
            }
        }
    } else {
        0 // Skip token counting if not requested
    };

    Ok(HttpResponse::Ok().json(MarkdownResponse {
        markdown,
        filename: filename.clone(),
        token_count,
    }))
}

/// Extracts text from PDF bytes
fn extract_text_from_pdf(data: &[u8]) -> Result<String, String> {
    match pdf_extract::extract_text_from_mem(data) {
        Ok(text) => {
            if text.trim().is_empty() {
                Err(
                    "PDF contains no extractable text (may be image-based or encrypted)"
                        .to_string(),
                )
            } else {
                Ok(text)
            }
        }
        Err(e) => Err(format!("Failed to extract text from PDF: {}", e)),
    }
}

/// Formats plain text as markdown
fn format_text_as_markdown(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut markdown = String::new();
    let mut prev_empty = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !prev_empty {
                markdown.push_str("\n\n");
                prev_empty = true;
            }
        } else {
            // Preserve the line, but ensure proper spacing
            markdown.push_str(trimmed);
            markdown.push('\n');
            prev_empty = false;
        }
    }

    markdown.trim().to_string()
}
