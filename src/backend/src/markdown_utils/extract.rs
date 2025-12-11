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
        if let Some(body_end_offset) = remaining_lower.find("</body>") {
            println!("✅ Found </body> tag at offset: {}", body_end_offset);

            // Extract content between opening and closing body tags
            let body_content = &remaining_html[..body_end_offset];
            println!("📦 Extracted body content: {} chars", body_content.len());

            // Return body content (even if it's just whitespace, let the converter handle it)
            return body_content.to_string();
        }

        println!("⚠️  No closing </body> tag found, returning everything after <body>");
        // If no closing tag found, return everything after the opening tag
        return remaining_html.to_string();
    }

    println!("⚠️  No <body tag found, returning original HTML");
    // If no body tag found, return the original HTML
    // This handles cases where the HTML is just body content or malformed
    html.to_string()
}
