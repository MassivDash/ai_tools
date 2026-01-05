use html_to_markdown_rs::{convert, ConversionOptions, PreprocessingPreset};

use crate::markdown_utils::clean::clean_markdown;
use crate::markdown_utils::extract::extract_body_content;
use crate::markdown_utils::links::extract_internal_links;

/// Options for HTML to Markdown conversion
#[derive(Debug, Clone)]
pub struct ConversionConfig {
    pub extract_body: bool,
    pub enable_preprocessing: bool,
    pub remove_navigation: bool,
    pub remove_forms: bool,
    pub preprocessing_preset: Option<String>, // "minimal", "standard", "aggressive"
    pub follow_links: bool, // Whether to follow internal links (for multi-document conversion)
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            extract_body: true,
            enable_preprocessing: false,
            remove_navigation: false,
            remove_forms: false,
            preprocessing_preset: None,
            follow_links: false,
        }
    }
}

/// Result of HTML to Markdown conversion
#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub markdown: String,
    pub internal_links: Vec<LinkInfo>,
}

/// Internal link information
#[derive(Debug, Clone)]
pub struct LinkInfo {
    pub original: String,  // The relative URL path
    pub full_url: String,  // The full absolute URL
    pub link_text: String, // The text/title from [text](url)
}

/// Converts HTML to Markdown with the given configuration
/// Returns the markdown content and internal links found
pub fn convert_html_to_markdown(
    html: &str,
    base_url: &str,
    config: &ConversionConfig,
) -> Result<ConversionResult, String> {
    // Extract body content if requested
    let html_to_convert = if config.extract_body {
        extract_body_content(html)
    } else {
        html.to_string()
    };

    // Configure conversion options
    let mut options = ConversionOptions {
        // Strip script, style, img, iframe, and other non-content tags
        strip_tags: vec![
            "script".to_string(),
            "style".to_string(),
            "img".to_string(),
            "iframe".to_string(),
            "noscript".to_string(),
            "object".to_string(),
            "embed".to_string(),
        ],
        ..Default::default()
    };

    // Enable preprocessing if requested
    if config.enable_preprocessing {
        options.preprocessing.enabled = true;

        // Set preprocessing preset
        match config.preprocessing_preset.as_deref() {
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

        options.preprocessing.remove_navigation = config.remove_navigation;
        options.preprocessing.remove_forms = config.remove_forms;
    }

    // Convert HTML to Markdown
    let markdown = convert(&html_to_convert, Some(options))
        .map_err(|e| format!("Failed to convert HTML to Markdown: {}", e))?;

    // Clean markdown: strip data URI images and remove artifacts
    let cleaned_markdown = clean_markdown(&markdown);

    // Extract internal links from cleaned markdown
    let internal_links = extract_internal_links(&cleaned_markdown, base_url);

    // Convert to LinkInfo format
    let link_info: Vec<LinkInfo> = internal_links
        .iter()
        .map(|link| LinkInfo {
            original: link.original.clone(),
            full_url: link.full_url.clone(),
            link_text: link.link_text.clone(),
        })
        .collect();

    Ok(ConversionResult {
        markdown: cleaned_markdown,
        internal_links: link_info,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_html_to_markdown_basic() {
        let html = r#"<html><body><h1>Hello</h1><p>World</p></body></html>"#;
        let base_url = "https://example.com";
        let config = ConversionConfig::default();

        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.markdown.contains("Hello"));
        assert!(result.markdown.contains("World"));
    }

    #[test]
    fn test_convert_html_to_markdown_without_extract_body() {
        let html = r#"<html><head><title>Test</title></head><body><p>Content</p></body></html>"#;
        let base_url = "https://example.com";
        let config = ConversionConfig {
            extract_body: false,
            ..Default::default()
        };

        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());
        let result = result.unwrap();
        // Should include head content when extract_body is false
        assert!(!result.markdown.is_empty());
    }

    #[test]
    fn test_convert_html_to_markdown_with_preprocessing() {
        let html = r#"<html><body><nav><a href="/">Home</a></nav><p>Content</p></body></html>"#;
        let base_url = "https://example.com";
        let config = ConversionConfig {
            enable_preprocessing: true,
            remove_navigation: true,
            ..Default::default()
        };

        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_html_to_markdown_preprocessing_presets() {
        let html = r#"<html><body><p>Content</p></body></html>"#;
        let base_url = "https://example.com";

        // Test minimal preset
        let mut config = ConversionConfig {
            enable_preprocessing: true,
            preprocessing_preset: Some("minimal".to_string()),
            ..Default::default()
        };
        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());

        // Test standard preset
        config.preprocessing_preset = Some("standard".to_string());
        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());

        // Test aggressive preset
        config.preprocessing_preset = Some("aggressive".to_string());
        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_html_to_markdown_strips_script_tags() {
        let html = r#"<html><body><script>alert('test');</script><p>Content</p></body></html>"#;
        let base_url = "https://example.com";
        let config = ConversionConfig::default();

        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());
        let result = result.unwrap();
        // Script tags should be stripped
        assert!(!result.markdown.contains("alert"));
    }

    #[test]
    fn test_conversion_config_default() {
        let config = ConversionConfig::default();
        assert!(config.extract_body);
        assert!(!config.enable_preprocessing);
        assert!(!config.remove_navigation);
        assert!(!config.remove_forms);
        assert!(!config.follow_links);
        assert_eq!(config.preprocessing_preset, None);
    }

    #[test]
    fn test_convert_html_to_markdown_with_remove_forms() {
        let html = r#"<html><body><form><input type="text"></form><p>Content</p></body></html>"#;
        let base_url = "https://example.com";
        let config = ConversionConfig {
            enable_preprocessing: true,
            remove_forms: true,
            ..Default::default()
        };

        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_html_to_markdown_with_remove_navigation() {
        let html = r#"<html><body><nav><a href="/">Home</a></nav><p>Content</p></body></html>"#;
        let base_url = "https://example.com";
        let config = ConversionConfig {
            enable_preprocessing: true,
            remove_navigation: true,
            ..Default::default()
        };

        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_html_to_markdown_invalid_preset() {
        let html = r#"<html><body><p>Content</p></body></html>"#;
        let base_url = "https://example.com";
        let config = ConversionConfig {
            enable_preprocessing: true,
            preprocessing_preset: Some("invalid_preset".to_string()),
            ..Default::default()
        };

        let result = convert_html_to_markdown(html, base_url, &config);
        // Should default to Minimal preset
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_html_to_markdown_empty_html() {
        let html = "";
        let base_url = "https://example.com";
        let config = ConversionConfig::default();

        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_html_to_markdown_with_internal_links() {
        let html = r#"<html><body><a href="/page">Link</a><p>Content</p></body></html>"#;
        let base_url = "https://example.com";
        let config = ConversionConfig::default();

        let result = convert_html_to_markdown(html, base_url, &config);
        assert!(result.is_ok());
        let result = result.unwrap();
        // Should extract internal links (at least 0, which is always true for len())
        assert!(result.internal_links.len() == 1); // Should find the /page link
    }
}
