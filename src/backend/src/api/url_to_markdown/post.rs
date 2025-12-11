use actix_web::{post, web, Error as ActixError, HttpResponse};
use html_to_markdown_rs::{convert, ConversionOptions, PreprocessingPreset};
use reqwest;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::markdown_utils::clean::clean_markdown;
use crate::markdown_utils::extract::extract_body_content;
use crate::markdown_utils::links::extract_internal_links;

#[derive(Deserialize, Serialize, Debug)]
pub struct UrlRequest {
    pub url: String,
    #[serde(default)]
    pub extract_body: bool,
    #[serde(default)]
    pub enable_preprocessing: bool,
    #[serde(default)]
    pub remove_navigation: bool,
    #[serde(default)]
    pub remove_forms: bool,
    #[serde(default)]
    pub preprocessing_preset: Option<String>, // "minimal", "aggressive", or None for default
}

#[derive(Serialize, Debug)]
pub struct LinkInfo {
    pub original: String,
    pub full_url: String,
}

#[derive(Serialize, Debug)]
pub struct MarkdownResponse {
    pub markdown: String,
    pub url: String,
    pub internal_links_count: usize,
    pub internal_links: Vec<LinkInfo>,
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
                    println!("📥 Received HTML from URL (length: {})", html_content.len());
                    // Limit response size to prevent stack overflow (10MB max)
                    const MAX_HTML_SIZE: usize = 10 * 1024 * 1024;
                    if html_content.len() > MAX_HTML_SIZE {
                        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                            "error": format!("HTML response too large: {} bytes (max {} bytes)", html_content.len(), MAX_HTML_SIZE)
                        })));
                    }

                    // Extract body content if requested
                    let html_to_convert = if body.extract_body {
                        println!("🔎 Extracting body content...");
                        extract_body_content(&html_content)
                    } else {
                        println!("📄 Using full HTML content (body extraction disabled)");
                        html_content
                    };

                    // Configure conversion options to strip unwanted tags and remove forms
                    let mut options = ConversionOptions::default();

                    // Strip script, style, img, iframe, and other non-content tags
                    options.strip_tags = vec![
                        "script".to_string(),
                        "style".to_string(),
                        "img".to_string(),
                        "iframe".to_string(),
                        "noscript".to_string(),
                        "object".to_string(),
                        "embed".to_string(),
                    ];

                    // Enable preprocessing if requested
                    if body.enable_preprocessing {
                        options.preprocessing.enabled = true;
                        
                        // Set preprocessing preset
                        match body.preprocessing_preset.as_deref() {
                            Some("minimal") => {
                                options.preprocessing.preset = PreprocessingPreset::Minimal;
                            }
                            Some("standard") => {
                                options.preprocessing.preset = PreprocessingPreset::Standard;
                            }
                            Some("aggressive") => {
                                options.preprocessing.preset = PreprocessingPreset::Aggressive;
                            }
                            _ => {
                                // Default preset
                                options.preprocessing.preset = PreprocessingPreset::Minimal;
                            }
                        }
                        
                        options.preprocessing.remove_navigation = body.remove_navigation;
                        options.preprocessing.remove_forms = body.remove_forms;
                        
                        println!("⚙️  Preprocessing enabled: preset={:?}, remove_navigation={}, remove_forms={}", 
                            options.preprocessing.preset, 
                            body.remove_navigation, 
                            body.remove_forms
                        );
                    } else {
                        println!("⚙️  Preprocessing disabled");
                    }

                    // Convert HTML to Markdown using html_to_markdown_rs
                    println!("🔄 Converting HTML to Markdown...");
                    match convert(&html_to_convert, Some(options)) {
                        Ok(markdown) => {
                            println!(
                                "✅ Conversion successful! Markdown length: {}",
                                markdown.len()
                            );

                            // Clean markdown: strip data URI images and remove artifacts
                            let cleaned_markdown = clean_markdown(&markdown);
                            println!(
                                "🧹 Cleaned markdown (removed data URI images and artifacts), new length: {}",
                                cleaned_markdown.len()
                            );

                            // Extract internal links from cleaned markdown
                            let internal_links = extract_internal_links(&cleaned_markdown, &url);
                            println!("🔗 Found {} internal links", internal_links.len());

                            // Convert InternalLink to LinkInfo for serialization
                            let link_info: Vec<LinkInfo> = internal_links
                                .iter()
                                .map(|link| LinkInfo {
                                    original: link.original.clone(),
                                    full_url: link.full_url.clone(),
                                })
                                .collect();

                            Ok(HttpResponse::Ok().json(MarkdownResponse {
                                markdown: cleaned_markdown,
                                url: url.clone(),
                                internal_links_count: link_info.len(),
                                internal_links: link_info,
                            }))
                        }
                        Err(error) => {
                            println!("❌ Conversion failed: {}", error);
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
                extract_body: true,
                enable_preprocessing: false,
                remove_navigation: false,
                remove_forms: false,
                preprocessing_preset: None,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_client_error());
    }
}
