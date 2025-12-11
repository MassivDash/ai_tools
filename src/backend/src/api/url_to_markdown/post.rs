use actix_web::{post, web, Error as ActixError, HttpResponse};
use html_to_markdown_rs::{convert, ConversionOptions, PreprocessingPreset};
use reqwest;
use serde::{Deserialize, Serialize};
use url::Url;

/// Extracts the body content from HTML, falling back to the full HTML if no body tag is found
fn extract_body_content(html: &str) -> String {
    // Try to find the body tag (case-insensitive search)
    let html_lower = html.to_lowercase();

    // Find opening body tag
    if let Some(body_start_idx) = html_lower.find("<body") {
        // Find the closing > of the opening body tag (handles attributes like <body class="...">)
        let body_tag_end = match html[body_start_idx..].find('>') {
            Some(pos) => body_start_idx + pos + 1,
            None => {
                // Malformed body tag, return original HTML
                return html.to_string();
            }
        };

        // Get the remaining HTML after the opening body tag
        let remaining_html = &html[body_tag_end..];
        let remaining_lower = &html_lower[body_tag_end..];

        // Find closing body tag (case-insensitive)
        if let Some(body_end_offset) = remaining_lower.find("</body>") {
            // Extract content between opening and closing body tags
            let body_content = &remaining_html[..body_end_offset];

            // Return body content (even if it's just whitespace, let the converter handle it)
            return body_content.to_string();
        }

        // If no closing tag found, return everything after the opening tag
        return remaining_html.to_string();
    }

    // If no body tag found, return the original HTML
    // This handles cases where the HTML is just body content or malformed
    html.to_string()
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UrlRequest {
    pub url: String,
}

#[derive(Serialize, Debug)]
pub struct MarkdownResponse {
    pub markdown: String,
    pub url: String,
}

#[post("/api/url-to-markdown")]
pub async fn convert_url_to_markdown(
    body: web::Json<UrlRequest>,
) -> Result<HttpResponse, ActixError> {
    let url = body.url.clone();

    // Validate URL format
    if Url::parse(&url).is_err() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid URL format"
        })));
    }

    // Fetch HTML from the URL
    let response = reqwest::get(&url).await;

    match response {
        Ok(response) => {
            if !response.status().is_success() {
                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                    "error": format!("Failed to fetch URL: HTTP {}", response.status())
                })));
            }

            let html = response.text().await;

            match html {
                Ok(html_content) => {
                    // Limit response size to prevent stack overflow (10MB max)
                    const MAX_HTML_SIZE: usize = 10 * 1024 * 1024;
                    if html_content.len() > MAX_HTML_SIZE {
                        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("HTML response too large: {} bytes (max {} bytes)", html_content.len(), MAX_HTML_SIZE)
                        })));
                    }

                    // Extract body content from HTML
                    let body_content = extract_body_content(&html_content);

                    // Configure conversion options to strip unwanted tags and use preprocessing
                    let mut options = ConversionOptions::default();

                    // Strip script, style, and other non-content tags
                    options.strip_tags = vec![
                        "script".to_string(),
                        "style".to_string(),
                        "noscript".to_string(),
                        "iframe".to_string(),
                        "object".to_string(),
                        "embed".to_string(),
                    ];

                    // Enable preprocessing for web scraping
                    options.preprocessing.enabled = true;
                    options.preprocessing.preset = PreprocessingPreset::Aggressive;
                    options.preprocessing.remove_navigation = true;
                    options.preprocessing.remove_forms = true;

                    // Convert HTML to Markdown using html_to_markdown_rs
                    match convert(&body_content, Some(options)) {
                        Ok(markdown) => Ok(HttpResponse::Ok().json(MarkdownResponse {
                            markdown,
                            url: url.clone(),
                        })),
                        Err(error) => {
                            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                                "error": format!("Failed to convert HTML to Markdown: {}", error)
                            })))
                        }
                    }
                }
                Err(error) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to read response body: {}", error)
                }))),
            }
        }
        Err(error) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Failed to fetch URL: {}", error)
        }))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_rt::test]
    async fn test_convert_url_to_markdown_invalid_url() {
        let mut app = test::init_service(App::new().service(convert_url_to_markdown)).await;

        let req = test::TestRequest::post()
            .uri("/api/url-to-markdown")
            .set_json(&UrlRequest {
                url: "not-a-valid-url".to_string(),
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_client_error());
    }
}
