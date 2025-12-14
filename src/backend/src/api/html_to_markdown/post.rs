use actix_web::{post, web, Error as ActixError, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::markdown_utils::convert::{convert_html_to_markdown, ConversionConfig};

#[derive(Deserialize, Serialize, Debug)]
pub struct HtmlRequest {
    pub html: String,
    #[serde(default)]
    pub extract_body: bool,
    #[serde(default)]
    pub enable_preprocessing: bool,
    #[serde(default)]
    pub remove_navigation: bool,
    #[serde(default)]
    pub remove_forms: bool,
    #[serde(default)]
    pub preprocessing_preset: Option<String>, // "minimal", "standard", "aggressive", or None for default
    #[serde(default)]
    pub count_tokens: bool, // Whether to count tokens (can be slow for large documents)
}

#[derive(Serialize, Debug)]
pub struct LinkInfo {
    pub original: String,
    pub full_url: String,
    pub link_text: String,
}

#[derive(Serialize, Debug)]
pub struct MarkdownResponse {
    pub markdown: String,
    pub internal_links_count: usize,
    pub internal_links: Vec<LinkInfo>,
    pub token_count: usize,
}

#[post("/api/html-to-markdown")]
pub async fn convert_html_to_markdown_endpoint(
    body: web::Json<HtmlRequest>,
) -> Result<HttpResponse, ActixError> {
    let html = body.html.clone();

    // Validate HTML is not empty
    if html.trim().is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "HTML content cannot be empty"
        })));
    }

    // Limit HTML size to prevent stack overflow (10MB max)
    const MAX_HTML_SIZE: usize = 10 * 1024 * 1024;
    if html.len() > MAX_HTML_SIZE {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("HTML content too large: {} bytes (max {} bytes)", html.len(), MAX_HTML_SIZE)
        })));
    }

    println!("📥 Received HTML (length: {})", html.len());

    // Build conversion config from request
    let config = ConversionConfig {
        extract_body: body.extract_body,
        enable_preprocessing: body.enable_preprocessing,
        remove_navigation: body.remove_navigation,
        remove_forms: body.remove_forms,
        preprocessing_preset: body.preprocessing_preset.clone(),
        follow_links: false, // Not applicable for direct HTML conversion
    };

    // Use a default base URL for link resolution (can be improved later)
    let base_url = "https://example.com";

    // Convert HTML to Markdown using the reusable function
    println!("🔄 Converting HTML to Markdown...");
    match convert_html_to_markdown(&html, base_url, &config) {
        Ok(result) => {
            println!(
                "✅ Conversion successful! Markdown length: {}, Links: {}",
                result.markdown.len(),
                result.internal_links.len()
            );

            // Count tokens in the markdown only if requested (can be slow for large documents)
            let token_count = if body.count_tokens {
                match crate::utils::tokenizer::count_tokens(&result.markdown) {
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
                markdown: result.markdown,
                internal_links_count: result.internal_links.len(),
                internal_links: result
                    .internal_links
                    .iter()
                    .map(|link| LinkInfo {
                        original: link.original.clone(),
                        full_url: link.full_url.clone(),
                        link_text: link.link_text.clone(),
                    })
                    .collect(),
                token_count,
            }))
        }
        Err(error) => {
            println!("❌ Conversion failed: {}", error);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": error
            })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_rt::test]
    async fn test_convert_html_to_markdown_empty_html() {
        let mut app = test::init_service(App::new().service(convert_html_to_markdown_endpoint)).await;

        let req = test::TestRequest::post()
            .uri("/api/html-to-markdown")
            .set_json(&HtmlRequest {
                html: "".to_string(),
                extract_body: true,
                enable_preprocessing: false,
                remove_navigation: false,
                remove_forms: false,
                preprocessing_preset: None,
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_client_error());
    }

    #[actix_rt::test]
    async fn test_convert_html_to_markdown_valid_html() {
        let mut app = test::init_service(App::new().service(convert_html_to_markdown_endpoint)).await;

        let req = test::TestRequest::post()
            .uri("/api/html-to-markdown")
            .set_json(&HtmlRequest {
                html: "<html><body><h1>Hello</h1><p>World</p></body></html>".to_string(),
                extract_body: true,
                enable_preprocessing: false,
                remove_navigation: false,
                remove_forms: false,
                preprocessing_preset: None,
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_rt::test]
    async fn test_convert_html_to_markdown_with_options() {
        let mut app = test::init_service(App::new().service(convert_html_to_markdown_endpoint)).await;

        let req = test::TestRequest::post()
            .uri("/api/html-to-markdown")
            .set_json(&HtmlRequest {
                html: "<html><body><h1>Hello</h1><p>World</p></body></html>".to_string(),
                extract_body: false,
                enable_preprocessing: true,
                remove_navigation: true,
                remove_forms: true,
                preprocessing_preset: Some("aggressive".to_string()),
                count_tokens: true,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }
}

