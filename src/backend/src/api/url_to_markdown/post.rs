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
    pub url: String,
    pub internal_links_count: usize,
    pub internal_links: Vec<LinkInfo>,
    pub token_count: usize,
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
                            // Count tokens in the markdown only if requested (can be slow for large documents)
                            let token_count = if body.count_tokens {
                                match crate::utils::tokenizer::count_tokens(&main_result.markdown) {
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
                                token_count,
                            }))
                        }
                        Err(error) => {
                            println!("Conversion failed: {}", error);
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

/// Helper function to create a safe and unique filename from link text or URL
fn create_unique_filename(
    link_text: &str,
    url: &str,
    used_filenames: &mut std::collections::HashSet<String>,
) -> String {
    // First, try to use the link text if it's meaningful
    let mut base_filename = if !link_text.is_empty() && link_text.len() < 100 {
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
            .and_then(|mut segments| segments.next_back())
            .unwrap_or("index")
            .to_string()
    };

    // Clean up the filename - remove any remaining invalid chars
    base_filename = base_filename
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect::<String>();

    if base_filename.is_empty() {
        base_filename = "index".to_string();
    }

    // Truncate if too long (leave room for counter suffix)
    if base_filename.len() > 90 {
        base_filename = base_filename.chars().take(90).collect();
    }

    // Remove .md extension if present (we'll add it later)
    let base_without_ext = if base_filename.ends_with(".md") {
        base_filename
            .strip_suffix(".md")
            .unwrap_or(&base_filename)
            .to_string()
    } else {
        base_filename
    };

    // Generate unique filename by appending counter if needed
    let mut filename = format!("{}.md", base_without_ext);
    let mut counter = 1;
    while used_filenames.contains(&filename) {
        filename = format!("{}_{}.md", base_without_ext, counter);
        counter += 1;
    }

    // Mark this filename as used
    used_filenames.insert(filename.clone());

    filename
}

/// Creates a zip file containing the main page and all internal links (1st level only)
async fn create_zip_with_links(
    main_url: &str,
    main_result: &crate::markdown_utils::convert::ConversionResult,
    config: &ConversionConfig,
) -> Result<Vec<u8>, String> {
    use std::collections::HashSet;
    use std::io::{Cursor, Write};
    use zip::write::{ExtendedFileOptions, FileOptions, ZipWriter};
    use zip::CompressionMethod;

    let zip_buffer = {
        let mut buffer = Vec::new();
        let mut zip_writer = ZipWriter::new(Cursor::new(&mut buffer));

        // Track used filenames to ensure uniqueness
        let mut used_filenames = HashSet::new();

        // Add main page to zip
        let main_filename = create_unique_filename("index", main_url, &mut used_filenames);
        println!("📄 Adding main page: {}", main_filename);
        zip_writer
            .start_file::<&str, ExtendedFileOptions>(
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
                                    let link_filename = create_unique_filename(
                                        &link.link_text,
                                        &link.full_url,
                                        &mut used_filenames,
                                    );
                                    println!(
                                        "✅ Adding link page: {} (from link text: '{}')",
                                        link_filename, link.link_text
                                    );

                                    // Verify content is not empty before writing
                                    if link_result.markdown.is_empty() {
                                        println!(
                                            "⚠️  Link {} produced empty markdown, skipping",
                                            link.full_url
                                        );
                                        continue;
                                    }

                                    zip_writer
                                        .start_file::<&str, ExtendedFileOptions>(
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

                                    println!(
                                        "✅ Successfully wrote {} bytes to {}",
                                        link_result.markdown.len(),
                                        link_filename
                                    );
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
        let app = test::init_service(App::new().service(convert_url_to_markdown)).await;

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
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_client_error());
    }

    #[actix_rt::test]
    async fn test_convert_url_to_markdown_valid_url_format() {
        let app = test::init_service(App::new().service(convert_url_to_markdown)).await;

        let req = test::TestRequest::post()
            .uri("/api/url-to-markdown")
            .set_json(&UrlRequest {
                url: "https://example.com".to_string(),
                extract_body: true,
                enable_preprocessing: false,
                remove_navigation: false,
                remove_forms: false,
                preprocessing_preset: None,
                follow_links: false,
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should not be 400 (bad request) for URL format - might fail on network but that's OK
        assert_ne!(resp.status().as_u16(), 400);
    }

    #[actix_rt::test]
    async fn test_convert_url_to_markdown_with_options() {
        let app = test::init_service(App::new().service(convert_url_to_markdown)).await;

        let req = test::TestRequest::post()
            .uri("/api/url-to-markdown")
            .set_json(&UrlRequest {
                url: "https://example.com".to_string(),
                extract_body: false,
                enable_preprocessing: true,
                remove_navigation: true,
                remove_forms: true,
                preprocessing_preset: Some("aggressive".to_string()),
                follow_links: false,
                count_tokens: true,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should not be 400 (bad request) for URL format
        assert_ne!(resp.status().as_u16(), 400);
    }

    #[actix_rt::test]
    async fn test_convert_url_to_markdown_rejects_an_unfetchable_host() {
        let app = test::init_service(App::new().service(convert_url_to_markdown)).await;

        // Parses as a URL, but the connection is always refused.
        let req = test::TestRequest::post()
            .uri("/api/url-to-markdown")
            .set_json(&UrlRequest {
                url: "http://127.0.0.1:1/page".to_string(),
                extract_body: true,
                enable_preprocessing: false,
                remove_navigation: false,
                remove_forms: false,
                preprocessing_preset: None,
                follow_links: false,
                count_tokens: false,
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 400);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert!(body["error"]
            .as_str()
            .unwrap()
            .starts_with("Failed to fetch URL:"));
    }

    #[actix_rt::test]
    async fn test_convert_url_to_markdown_rejects_urls_without_a_scheme() {
        let app = test::init_service(App::new().service(convert_url_to_markdown)).await;

        for url in ["", "example.com/page", "//example.com", "   "] {
            let req = test::TestRequest::post()
                .uri("/api/url-to-markdown")
                .set_json(&UrlRequest {
                    url: url.to_string(),
                    extract_body: true,
                    enable_preprocessing: false,
                    remove_navigation: false,
                    remove_forms: false,
                    preprocessing_preset: None,
                    follow_links: false,
                    count_tokens: false,
                })
                .to_request();

            let resp = test::call_service(&app, req).await;
            assert_eq!(
                resp.status().as_u16(),
                400,
                "url {:?} should be rejected",
                url
            );
            let body: serde_json::Value = test::read_body_json(resp).await;
            assert_eq!(body["error"], "Invalid URL format");
        }
    }

    // ---- create_zip_with_links ----

    fn config() -> ConversionConfig {
        ConversionConfig {
            extract_body: true,
            enable_preprocessing: false,
            remove_navigation: false,
            remove_forms: false,
            preprocessing_preset: None,
            follow_links: true,
        }
    }

    fn link(text: &str, url: &str) -> crate::markdown_utils::convert::LinkInfo {
        crate::markdown_utils::convert::LinkInfo {
            original: url.to_string(),
            full_url: url.to_string(),
            link_text: text.to_string(),
        }
    }

    fn zip_entry_names(bytes: &[u8]) -> Vec<String> {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.to_vec())).unwrap();
        (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_string())
            .collect()
    }

    #[actix_rt::test]
    async fn test_create_zip_with_links_writes_the_main_page() {
        let main = crate::markdown_utils::convert::ConversionResult {
            markdown: "# Main page\n\nBody text.".to_string(),
            internal_links: Vec::new(),
        };

        let zip_bytes = create_zip_with_links("https://example.com/docs", &main, &config())
            .await
            .unwrap();

        assert_eq!(zip_entry_names(&zip_bytes), vec!["index.md"]);

        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
        let mut contents = String::new();
        std::io::Read::read_to_string(&mut archive.by_index(0).unwrap(), &mut contents).unwrap();
        assert_eq!(contents, "# Main page\n\nBody text.");
    }

    #[actix_rt::test]
    async fn test_create_zip_with_links_skips_links_it_cannot_fetch() {
        let main = crate::markdown_utils::convert::ConversionResult {
            markdown: "# Main".to_string(),
            internal_links: vec![
                link("Refused", "http://127.0.0.1:1/one"),
                link("Also refused", "http://127.0.0.1:1/two"),
            ],
        };

        let zip_bytes = create_zip_with_links("https://example.com/docs", &main, &config())
            .await
            .unwrap();

        // Unfetchable links are logged and skipped; the archive still contains
        // the main page rather than failing outright.
        assert_eq!(zip_entry_names(&zip_bytes), vec!["index.md"]);
    }

    #[actix_rt::test]
    async fn test_create_zip_with_links_deduplicates_repeated_urls() {
        let main = crate::markdown_utils::convert::ConversionResult {
            markdown: "# Main".to_string(),
            internal_links: vec![
                link("First", "http://127.0.0.1:1/same"),
                link("Second", "http://127.0.0.1:1/same"),
            ],
        };

        // Both entries point at the same URL, so only one fetch is attempted.
        let zip_bytes = create_zip_with_links("https://example.com/docs", &main, &config())
            .await
            .unwrap();

        assert_eq!(zip_entry_names(&zip_bytes), vec!["index.md"]);
    }

    #[actix_rt::test]
    async fn test_create_zip_with_links_skips_the_main_url_when_it_appears_as_a_link() {
        let main_url = "https://example.com/docs";
        let main = crate::markdown_utils::convert::ConversionResult {
            markdown: "# Main".to_string(),
            internal_links: vec![link("Self", main_url)],
        };

        let zip_bytes = create_zip_with_links(main_url, &main, &config())
            .await
            .unwrap();

        assert_eq!(zip_entry_names(&zip_bytes), vec!["index.md"]);
    }
}

/// `create_unique_filename` is synchronous, so its tests live outside the module
/// that imports `actix_web::test` (which shadows the built-in `#[test]`).
#[cfg(test)]
mod filename_tests {
    use super::*;

    // ---- create_unique_filename ----

    fn unique(link_text: &str, url: &str, used: &mut std::collections::HashSet<String>) -> String {
        create_unique_filename(link_text, url, used)
    }

    #[test]
    fn test_create_unique_filename_slugifies_the_link_text() {
        let mut used = std::collections::HashSet::new();
        assert_eq!(
            unique("About Us", "https://example.com/x", &mut used),
            "about_us.md"
        );
        // Every non-alphanumeric character becomes an underscore, including the
        // separators around the slash and the trailing "!".
        assert_eq!(
            unique("Contact / Support!", "https://example.com/y", &mut used),
            "contact___support_.md"
        );
    }

    #[test]
    fn test_create_unique_filename_appends_a_counter_on_collisions() {
        let mut used = std::collections::HashSet::new();
        assert_eq!(
            unique("Docs", "https://example.com/a", &mut used),
            "docs.md"
        );
        assert_eq!(
            unique("Docs", "https://example.com/b", &mut used),
            "docs_1.md"
        );
        assert_eq!(
            unique("Docs", "https://example.com/c", &mut used),
            "docs_2.md"
        );
        assert_eq!(used.len(), 3);
    }

    #[test]
    fn test_create_unique_filename_falls_back_to_the_last_url_segment() {
        let mut used = std::collections::HashSet::new();

        // Empty link text: use the URL path instead.
        assert_eq!(
            unique("", "https://example.com/docs/getting-started", &mut used),
            "getting-started.md"
        );

        // Link text of 100 chars or more is also treated as unusable.
        let long_text = "x".repeat(100);
        assert_eq!(
            unique(&long_text, "https://example.com/guide/setup", &mut used),
            "setup.md"
        );
    }

    #[test]
    fn test_create_unique_filename_falls_back_to_index() {
        let mut used = std::collections::HashSet::new();

        // A root URL has an empty last path segment.
        assert_eq!(unique("", "https://example.com/", &mut used), "index.md");
        // Whitespace-only link text slugifies to nothing.
        assert_eq!(
            unique("   ", "https://example.com/", &mut used),
            "index_1.md"
        );
        // An unparseable fallback URL.
        assert_eq!(unique("", "not a url", &mut used), "index_2.md");
    }

    #[test]
    fn test_create_unique_filename_truncates_very_long_slugs() {
        let mut used = std::collections::HashSet::new();
        // 99 alphanumeric chars: usable as link text but longer than the 90-char cap.
        let name = unique(&"a".repeat(99), "https://example.com/x", &mut used);

        assert_eq!(name, format!("{}.md", "a".repeat(90)));
    }

    #[test]
    fn test_create_unique_filename_does_not_double_the_md_extension() {
        let mut used = std::collections::HashSet::new();

        // The ".md" strip only ever applies to names taken from the URL path -
        // link text has its dots replaced by underscores before that check runs.
        assert_eq!(
            unique("", "https://example.com/docs/readme.md", &mut used),
            "readme.md"
        );
        assert_eq!(
            unique("readme.md", "https://example.com/x", &mut used),
            "readme_md.md"
        );
    }

    #[test]
    fn test_create_unique_filename_slug_keeps_punctuation_as_underscores() {
        let mut used = std::collections::HashSet::new();

        // Non-alphanumerics all collapse to underscores rather than being dropped.
        assert_eq!(
            unique("!!!", "https://example.com/page", &mut used),
            "___.md"
        );
    }
}
