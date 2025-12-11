use regex::Regex;
use url::Url;

/// Represents an internal link found in markdown
#[derive(Debug, Clone)]
pub struct InternalLink {
    pub original: String,  // The relative URL path
    pub full_url: String,  // The full absolute URL
    pub link_text: String, // The text/title from [text](url)
}

/// Extracts internal links from markdown content and converts them to full URLs
/// Internal links are identified as relative links (starting with / or ./ or ../)
/// that don't start with http:// or https://
pub fn extract_internal_links(markdown: &str, base_url: &str) -> Vec<InternalLink> {
    // Parse the base URL to get the origin
    let base = match Url::parse(base_url) {
        Ok(url) => url,
        Err(_) => {
            println!("⚠️  Failed to parse base URL: {}", base_url);
            return Vec::new();
        }
    };

    let origin = format!("{}://{}", base.scheme(), base.host_str().unwrap_or(""));

    // Regex to match markdown links: [text](url)
    // This will match both [text](url) and [text](url "title") formats
    let link_regex = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();

    let mut internal_links: Vec<InternalLink> = Vec::new();

    for cap in link_regex.captures_iter(markdown) {
        let link_text = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        let link_url = cap.get(2).map(|m| m.as_str().trim()).unwrap_or("");

        // Skip empty links, anchors (#), mailto:, tel:, and external links
        if link_url.is_empty()
            || link_url.starts_with('#')
            || link_url.starts_with("mailto:")
            || link_url.starts_with("tel:")
            || link_url.starts_with("http://")
            || link_url.starts_with("https://")
        {
            continue;
        }

        // Skip circular/self-referential links like [/](/) or [2D Shapes](/)
        // These are links that point to the root path "/" which are typically navigation elements
        if link_url == "/" {
            continue;
        }

        // Check if it's a relative link (internal link)
        if link_url.starts_with('/') || link_url.starts_with("./") || link_url.starts_with("../") {
            // Build full URL
            let full_url = if link_url.starts_with('/') {
                // Absolute path from root
                format!("{}{}", origin, link_url)
            } else {
                // Relative path - resolve against base URL
                match base.join(link_url) {
                    Ok(joined_url) => joined_url.to_string(),
                    Err(_) => {
                        println!(
                            "⚠️  Failed to join base URL with relative path: {}",
                            link_url
                        );
                        continue;
                    }
                }
            };

            internal_links.push(InternalLink {
                original: link_url.to_string(),
                full_url,
                link_text: link_text.to_string(),
            });
        }
    }

    // Remove duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    internal_links
        .into_iter()
        .filter(|link| seen.insert(link.full_url.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_internal_links() {
        let markdown = r#"
# Test Document

[Internal Link 1](/page1)
[Internal Link 2](./page2)
[Internal Link 3](../page3)
[External Link](https://example.com)
[Anchor Link](#section)
[Another Internal](/about)
[/](/)
[Home](/)
"#;

        let base_url = "https://example.com/docs";
        let links = extract_internal_links(markdown, base_url);

        // Should have 4 links (circular links [/](/) and [Home](/) pointing to root should be filtered out)
        assert_eq!(links.len(), 4);
        assert!(
            links
                .iter()
                .any(|l| l.full_url == "https://example.com/page1"
                    && l.link_text == "Internal Link 1")
        );
        assert!(links
            .iter()
            .any(|l| l.full_url == "https://example.com/docs/page2"
                && l.link_text == "Internal Link 2"));

        // Verify circular links are not included
        assert!(!links.iter().any(|l| l.full_url == "https://example.com/"));
    }
}
