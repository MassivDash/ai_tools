/// Extracts the body content from HTML, falling back to the full HTML if no body tag is found
pub fn extract_body_content(html: &str) -> String {
    println!(
        "🔎 Extracting body content from HTML (total length: {})",
        html.len()
    );

    // Try to find the body tag (case-insensitive search)
    let html_lower = html.to_lowercase();

    // Find opening body tag
    if let Some(body_start_idx) = html_lower.find("<body") {
        println!("✅ Found <body tag at index: {}", body_start_idx);

        // Find the closing > of the opening body tag (handles attributes like <body class="...">)
        let body_tag_end = match html[body_start_idx..].find('>') {
            Some(pos) => {
                let end = body_start_idx + pos + 1;
                println!("✅ Found closing > of body tag at index: {}", end);
                end
            }
            None => {
                println!("⚠️  Malformed body tag (no closing >), returning original HTML");
                return html.to_string();
            }
        };

        // Get the remaining HTML after the opening body tag
        let remaining_html = &html[body_tag_end..];
        let remaining_lower = &html_lower[body_tag_end..];

        println!(
            "📏 Remaining HTML after body tag: {} chars",
            remaining_html.len()
        );

        // Find closing body tag (case-insensitive)
        // Note: HTML5 allows omitting the closing </body> tag, so many modern sites don't have it
        // Search for </body> in the lowercase version (handles </BODY>, </Body>, etc.)
        if let Some(body_end_offset) = remaining_lower.find("</body>") {
            println!("✅ Found </body> tag at offset: {}", body_end_offset);

            // Extract content between opening and closing body tags
            let body_content = &remaining_html[..body_end_offset];
            println!("📦 Extracted body content: {} chars", body_content.len());

            // Return body content (even if it's just whitespace, let the converter handle it)
            return body_content.to_string();
        }

        // No closing body tag found - this is valid HTML5!
        // HTML5 specification allows omitting closing tags for html, head, body, and p elements
        // Browsers automatically close these tags, so many modern websites don't include </body>
        println!("⚠️  No closing </body> tag found");

        // Return everything after the opening tag - this is what browsers do
        return remaining_html.to_string();
    }

    println!("⚠️  No <body tag found, returning original HTML");
    // If no body tag found, return the original HTML
    // This handles cases where the HTML is just body content or malformed
    html.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_body_content_with_body_tags() {
        let html =
            r#"<html><head><title>Test</title></head><body><p>Hello World</p></body></html>"#;
        let result = extract_body_content(html);
        assert!(result.contains("Hello World"));
        assert!(!result.contains("<html>"));
        assert!(!result.contains("<head>"));
    }

    #[test]
    fn test_extract_body_content_no_closing_body_tag() {
        let html = r#"<html><head><title>Test</title></head><body><p>Hello World</p>"#;
        let result = extract_body_content(html);
        assert!(result.contains("Hello World"));
    }

    #[test]
    fn test_extract_body_content_no_body_tag() {
        let html = r#"<p>Hello World</p>"#;
        let result = extract_body_content(html);
        assert_eq!(result, html);
    }

    #[test]
    fn test_extract_body_content_with_attributes() {
        let html = r#"<html><body class="main"><p>Hello World</p></body></html>"#;
        let result = extract_body_content(html);
        assert!(result.contains("Hello World"));
        assert!(!result.contains("class=\"main\""));
    }

    #[test]
    fn test_extract_body_content_malformed_body_tag() {
        // Test with body tag that has no closing > before other tags
        // In this case, <body<p> will find <body, then find > from <p>, so it extracts content
        // This is actually valid behavior - it extracts what it can
        let html = r#"<html><body<p>Hello</p></body></html>"#;
        let result = extract_body_content(html);
        // The function finds <body, then finds > from <p>, so it extracts "Hello</p>"
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_extract_body_content_empty_body() {
        let html = r#"<html><body></body></html>"#;
        let result = extract_body_content(html);
        assert_eq!(result, "");
    }

    #[test]
    fn test_extract_body_content_case_insensitive() {
        let html = r#"<HTML><BODY><p>Hello World</p></BODY></HTML>"#;
        let result = extract_body_content(html);
        assert!(result.contains("Hello World"));
    }

    #[test]
    fn test_extract_body_content_body_with_whitespace() {
        let html = r#"<html><body class="main" id="page"><p>Hello</p></body></html>"#;
        let result = extract_body_content(html);
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_extract_body_content_nested_tags() {
        let html = r#"<html><body><div><p>Nested content</p></div></body></html>"#;
        let result = extract_body_content(html);
        assert!(result.contains("Nested content"));
    }

    #[test]
    fn test_extract_body_content_only_opening_tag() {
        // HTML5 allows omitting closing body tag
        let html = r#"<html><body><p>Content without closing body</p>"#;
        let result = extract_body_content(html);
        assert!(result.contains("Content without closing body"));
    }
}
