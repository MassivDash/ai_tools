use actix_web::{post, web, Error as ActixError, HttpResponse};
use reqwest;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::markdown_utils::convert::{convert_html_to_markdown, ConversionConfig};

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
    pub preprocessing_preset: Option<String>, // "minimal", "standard", "aggressive", or None for default
    #[serde(default)]
    pub follow_links: bool, // Whether to follow internal links and create zip file
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

                    // Build conversion config from request
                    let config = ConversionConfig {
                        extract_body: body.extract_body,
                        enable_preprocessing: body.enable_preprocessing,
                        remove_navigation: body.remove_navigation,
                        remove_forms: body.remove_forms,
                        preprocessing_preset: body.preprocessing_preset.clone(),
                        follow_links: body.follow_links,
                    };

                    // Convert HTML to Markdown using the reusable function
                    println!("🔄 Converting HTML to Markdown...");
                    match convert_html_to_markdown(&html_content, &url, &config) {
                        Ok(main_result) => {
                            println!(
                                "✅ Main page conversion successful! Markdown length: {}, Links: {}",
                                main_result.markdown.len(),
                                main_result.internal_links.len()
                            );

                            // If follow_links is enabled, convert all internal links and create zip
                            if body.follow_links && !main_result.internal_links.is_empty() {
                                println!(
                                    "🔗 Following {} internal links...",
                                    main_result.internal_links.len()
                                );

                                match create_zip_with_links(&url, &main_result, &config).await {
                                    Ok(zip_data) => {
                                        println!(
                                            "✅ Created zip file with {} bytes",
                                            zip_data.len()
                                        );

                                        // Return zip file as binary response
                                        return Ok(HttpResponse::Ok()
                                            .content_type("application/zip")
                                            .append_header(("Content-Disposition", format!("attachment; filename=\"markdown_archive_{}.zip\"", 
                                                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())))
                                            .body(zip_data));
                                    }
                                    Err(e) => {
                                        println!("⚠️  Failed to create zip file: {}, returning main page only", e);
                                        // Fall through to return main page only
                                    }
                                }
                            }

                            // Return single page result (either follow_links disabled or zip creation failed)
                            Ok(HttpResponse::Ok().json(MarkdownResponse {
                                markdown: main_result.markdown,
                                url: url.clone(),
                                internal_links_count: main_result.internal_links.len(),
                                internal_links: main_result
                                    .internal_links
                                    .iter()
                                    .map(|link| LinkInfo {
                                        original: link.original.clone(),
                                        full_url: link.full_url.clone(),
                                        link_text: link.link_text.clone(),
                                    })
                                    .collect(),
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

/// Creates a zip file containing the main page and all internal links (1st level only)
async fn create_zip_with_links(
    main_url: &str,
    main_result: &crate::markdown_utils::convert::ConversionResult,
    config: &ConversionConfig,
) -> Result<Vec<u8>, String> {
    use std::collections::HashSet;
    use std::io::{Cursor, Write};
    use zip::write::{FileOptions, ZipWriter};
    use zip::CompressionMethod;

    let zip_buffer = {
        let mut buffer = Vec::new();
        let mut zip_writer = ZipWriter::new(Cursor::new(&mut buffer));

        // Helper function to create a safe filename from link text or URL
        let create_filename = |link_text: &str, url: &str| -> String {
            // First, try to use the link text if it's meaningful
            let mut filename = if !link_text.is_empty() && link_text.len() < 100 {
                // Sanitize link text: remove special chars, keep alphanumeric, spaces, hyphens, underscores
                link_text
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                            c
                        } else if c.is_whitespace() {
                            ' '
                        } else {
                            '_'
                        }
                    })
                    .collect::<String>()
                    .split_whitespace()
                    .collect::<Vec<&str>>()
                    .join("_")
                    .to_lowercase()
            } else {
                // Fall back to URL path if link text is empty or too long
                let parsed = Url::parse(url).ok();
                parsed
                    .as_ref()
                    .and_then(|u| u.path_segments())
                    .and_then(|segments| segments.last())
                    .unwrap_or("index")
                    .to_string()
            };

            // Clean up the filename - remove any remaining invalid chars
            filename = filename
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
                .collect::<String>();

            if filename.is_empty() {
                filename = "index".to_string();
            }

            // Truncate if too long
            if filename.len() > 100 {
                filename = filename.chars().take(100).collect();
            }

            // Ensure .md extension
            if !filename.ends_with(".md") {
                filename.push_str(".md");
            }

            filename
        };

        // Add main page to zip
        let main_filename = create_filename("index", main_url);
        println!("📄 Adding main page: {}", main_filename);
        zip_writer
            .start_file(
                &main_filename,
                FileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .map_err(|e| format!("Failed to create zip entry: {}", e))?;
        zip_writer
            .write_all(main_result.markdown.as_bytes())
            .map_err(|e| format!("Failed to write to zip: {}", e))?;

        // Track processed URLs to avoid duplicates
        let mut processed_urls = HashSet::new();
        processed_urls.insert(main_url.to_string());

        // Convert each internal link (1st level only - follow_links flag prevents deeper recursion)
        let mut config_no_follow = config.clone();
        config_no_follow.follow_links = false; // Prevent recursive following

        for (idx, link) in main_result.internal_links.iter().enumerate() {
            // Skip if already processed
            if processed_urls.contains(&link.full_url) {
                continue;
            }
            processed_urls.insert(link.full_url.clone());

            println!(
                "🔗 [{}/{}] Converting link: {}",
                idx + 1,
                main_result.internal_links.len(),
                link.full_url
            );

            // Fetch HTML from the link
            match reqwest::get(&link.full_url).await {
                Ok(response) => {
                    if !response.status().is_success() {
                        println!(
                            "⚠️  Failed to fetch {}: HTTP {}",
                            link.full_url,
                            response.status()
                        );
                        continue;
                    }

                    match response.text().await {
                        Ok(link_html) => {
                            // Limit size
                            if link_html.len() > 10 * 1024 * 1024 {
                                println!("⚠️  Link {} too large, skipping", link.full_url);
                                continue;
                            }

                            // Convert to markdown
                            match convert_html_to_markdown(
                                &link_html,
                                &link.full_url,
                                &config_no_follow,
                            ) {
                                Ok(link_result) => {
                                    let link_filename =
                                        create_filename(&link.link_text, &link.full_url);
                                    println!(
                                        "✅ Adding link page: {} (from link text: '{}')",
                                        link_filename, link.link_text
                                    );

                                    zip_writer
                                        .start_file(
                                            &link_filename,
                                            FileOptions::default()
                                                .compression_method(CompressionMethod::Deflated),
                                        )
                                        .map_err(|e| {
                                            format!("Failed to create zip entry: {}", e)
                                        })?;
                                    zip_writer
                                        .write_all(link_result.markdown.as_bytes())
                                        .map_err(|e| format!("Failed to write to zip: {}", e))?;
                                }
                                Err(e) => {
                                    println!("⚠️  Failed to convert link {}: {}", link.full_url, e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("⚠️  Failed to read response from {}: {}", link.full_url, e);
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Failed to fetch {}: {}", link.full_url, e);
                }
            }
        }

        // Finish zip file and extract the buffer from the Cursor
        let cursor = zip_writer
            .finish()
            .map_err(|e| format!("Failed to finish zip file: {}", e))?;

        // Extract the buffer from the Cursor - clone since into_inner() returns &mut
        cursor.into_inner().clone()
    };

    Ok(zip_buffer)
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
                follow_links: false,
            })
            .to_request();

        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_client_error());
    }
}
