use actix_multipart::Multipart;
use actix_web::{post, Error as ActixError, HttpResponse};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
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

        if field_name == Some("file") {
            // Get filename from content disposition
            let content_disposition = field.content_disposition();
            if let Some(name) = content_disposition
                .as_ref()
                .and_then(|cd| cd.get_filename())
            {
                filename = Some(name.to_string());
            }

            // Read file data
            let mut data = Vec::new();
            while let Some(chunk) = field.try_next().await? {
                data.extend_from_slice(&chunk);
            }
            file_data = Some(data);
        } else if field_name == Some("count_tokens") {
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
            println!("Failed to extract text from PDF: {}", e);
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

/// Extracts text from PDF bytes using pdftotext (external tool)
fn extract_text_from_pdf(data: &[u8]) -> Result<String, String> {
    use std::io::Write;
    use std::process::Command;
    use tempfile::NamedTempFile;

    // Create a temporary file to write the PDF data to
    let mut temp_file =
        NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {}", e))?;
    temp_file
        .write_all(data)
        .map_err(|e| format!("Failed to write PDF data: {}", e))?;

    let temp_path = temp_file.path().to_owned();

    // Call pdftotext
    let output = Command::new("pdftotext")
        .arg("-layout") // Maintain layout
        .arg("-enc")
        .arg("UTF-8")
        .arg(&temp_path)
        .arg("-") // Output to stdout
        .output()
        .map_err(|e| format!("Failed to execute pdftotext: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftotext failed: {}", stderr));
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 output from pdftotext: {}", e))?;

    if text.trim().is_empty() {
        return Err(
            "PDF contains no extractable text (may be image-based or encrypted)".to_string(),
        );
    }

    Ok(text)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_text_as_markdown() {
        let text = "Line 1\n\nLine 2\nLine 3\n\n\nLine 4";
        let markdown = format_text_as_markdown(text);

        // Should preserve lines and normalize spacing
        assert!(markdown.contains("Line 1"));
        assert!(markdown.contains("Line 2"));
        assert!(markdown.contains("Line 3"));
        assert!(markdown.contains("Line 4"));
    }

    #[test]
    fn test_format_text_as_markdown_empty() {
        let text = "";
        let markdown = format_text_as_markdown(text);
        assert_eq!(markdown, "");
    }

    #[test]
    fn test_format_text_as_markdown_whitespace() {
        let text = "   \n\n   \n  ";
        let markdown = format_text_as_markdown(text);
        // Should trim and normalize
        assert_eq!(markdown, "");
    }

    #[test]
    fn test_format_text_as_markdown_single_line() {
        let text = "Single line";
        let markdown = format_text_as_markdown(text);
        assert_eq!(markdown, "Single line");
    }

    #[test]
    fn test_format_text_as_markdown_multiple_empty_lines() {
        let text = "Line 1\n\n\n\nLine 2";
        let markdown = format_text_as_markdown(text);
        // Should normalize multiple empty lines to double newline
        assert!(markdown.contains("Line 1"));
        assert!(markdown.contains("Line 2"));
    }

    #[test]
    fn test_format_text_as_markdown_preserves_content() {
        let text = "First paragraph\n\nSecond paragraph\nWith multiple lines";
        let markdown = format_text_as_markdown(text);
        assert!(markdown.contains("First paragraph"));
        assert!(markdown.contains("Second paragraph"));
        assert!(markdown.contains("With multiple lines"));
    }

    #[test]
    fn test_extract_text_from_pdf_empty_data() {
        // Test with empty/invalid PDF data
        let empty_data = b"";
        let result = extract_text_from_pdf(empty_data);
        // Should return an error for empty data
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_from_pdf_invalid_data() {
        // Test with invalid PDF data (not a real PDF)
        let invalid_data = b"This is not a PDF file";
        let result = extract_text_from_pdf(invalid_data);
        // Should return an error for invalid PDF
        assert!(result.is_err());
    }

    #[test]
    fn test_format_text_as_markdown_trims_leading_and_trailing_blank_lines() {
        assert_eq!(format_text_as_markdown("\n\n  body  \n\n"), "body");
    }

    #[test]
    fn test_format_text_as_markdown_collapses_runs_of_blank_lines() {
        // A run of blank lines emits a single "\n\n", but the preceding content
        // line already contributed its own trailing "\n", so paragraphs end up
        // separated by three newlines rather than two.
        assert_eq!(format_text_as_markdown("a\n\n\n\n\nb"), "a\n\n\nb");
        assert_eq!(format_text_as_markdown("a\n\nb"), "a\n\n\nb");
    }

    #[test]
    fn test_format_text_as_markdown_trims_each_line_independently() {
        let markdown = format_text_as_markdown("   indented\n\t\ttabbed   ");
        assert_eq!(markdown, "indented\ntabbed");
    }
}

/// Endpoint-level tests live in their own module: importing `actix_web::test`
/// shadows the built-in `#[test]` attribute used by the tests above.
///
/// These cover the request-validation paths only. Anything past validation calls
/// out to the `pdftotext` binary, which is deliberately left alone here.
#[cfg(test)]
mod endpoint_tests {
    use super::*;
    use actix_web::{test, App};

    const BOUNDARY: &str = "----------------pdftest";

    fn multipart_body(parts: &[(&str, Option<&str>, &[u8])]) -> Vec<u8> {
        let mut body = Vec::new();
        for (name, filename, content) in parts {
            body.extend_from_slice(format!("--{}\r\n", BOUNDARY).as_bytes());
            match filename {
                Some(filename) => body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n\
                         Content-Type: application/octet-stream\r\n\r\n",
                        name, filename
                    )
                    .as_bytes(),
                ),
                None => body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", name).as_bytes(),
                ),
            }
            body.extend_from_slice(content);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{}--\r\n", BOUNDARY).as_bytes());
        body
    }

    async fn post(parts: &[(&str, Option<&str>, &[u8])]) -> (u16, serde_json::Value) {
        let app = test::init_service(App::new().service(convert_pdf_to_markdown)).await;

        let req = test::TestRequest::post()
            .uri("/api/pdf-to-markdown")
            .insert_header((
                "content-type",
                format!("multipart/form-data; boundary={}", BOUNDARY),
            ))
            .set_payload(multipart_body(parts))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status().as_u16();
        (status, test::read_body_json(resp).await)
    }

    #[actix_web::test]
    async fn test_no_file_field_is_rejected() {
        let (status, body) = post(&[("count_tokens", None, b"false")]).await;

        assert_eq!(status, 400);
        assert_eq!(body["error"], "No file provided");
    }

    #[actix_web::test]
    async fn test_form_whose_only_field_is_not_a_file_is_rejected() {
        let (status, body) = post(&[("metadata", None, b"{}")]).await;

        assert_eq!(status, 400);
        assert_eq!(body["error"], "No file provided");
    }

    #[actix_web::test]
    async fn test_zero_byte_file_is_rejected() {
        let (status, body) = post(&[("file", Some("empty.pdf"), b"")]).await;

        // Distinct from "No file provided": the field was there but carried nothing.
        assert_eq!(status, 400);
        assert_eq!(body["error"], "No file data received");
    }

    #[actix_web::test]
    async fn test_non_pdf_filename_is_rejected() {
        let (status, body) = post(&[("file", Some("notes.txt"), b"some text")]).await;

        assert_eq!(status, 400);
        assert_eq!(body["error"], "File must be a PDF");
    }

    #[actix_web::test]
    async fn test_count_tokens_field_is_parsed_before_validation_fails() {
        // Exercises the count_tokens branch of the multipart loop; the request
        // still fails validation because the file is not a PDF.
        for flag in ["true", "TRUE", "1", "false", "0", "nonsense"] {
            let (status, body) = post(&[
                ("count_tokens", None, flag.as_bytes()),
                ("file", Some("notes.md"), b"# heading"),
            ])
            .await;

            assert_eq!(status, 400, "flag {} should not change validation", flag);
            assert_eq!(body["error"], "File must be a PDF");
        }
    }

    #[actix_web::test]
    async fn test_unknown_fields_are_ignored() {
        let (status, body) = post(&[
            ("unexpected", None, b"whatever"),
            ("file", Some("doc.docx"), b"binary"),
        ])
        .await;

        assert_eq!(status, 400);
        assert_eq!(body["error"], "File must be a PDF");
    }
}
