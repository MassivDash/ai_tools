use regex::Regex;

/// Removes data URI images from markdown content
/// Strips markdown image syntax like ![alt](data:image/...) or ![alt](data:image/svg+xml;base64,...)
pub fn strip_data_uri_images(markdown: &str) -> String {
    // Regex to match markdown images with data URIs
    // Matches: ![alt text](data:image/...)
    // This will match data URIs for any image type (png, jpeg, svg, gif, etc.)
    let data_uri_image_regex = Regex::new(r"!\[([^\]]*)\]\(data:image/[^)]+\)").unwrap();
    
    let cleaned = data_uri_image_regex.replace_all(markdown, "");
    
    cleaned.to_string()
}

/// Cleans up malformed markdown content
/// Removes duplicate brackets, empty links, and other artifacts
pub fn clean_markdown_artifacts(markdown: &str) -> String {
    let mut cleaned = markdown.to_string();
    
    // Remove multiple consecutive opening brackets (like [[[[[[[)
    let multiple_brackets_regex = Regex::new(r"\[{3,}").unwrap();
    cleaned = multiple_brackets_regex.replace_all(&cleaned, "").to_string();
    
    // Remove multiple consecutive closing brackets
    let multiple_closing_brackets_regex = Regex::new(r"\]{3,}").unwrap();
    cleaned = multiple_closing_brackets_regex.replace_all(&cleaned, "").to_string();
    
    // Remove empty markdown links: [](url) or [text]()
    let empty_links_regex = Regex::new(r"\[\]\([^)]*\)|\[[^\]]*\]\(\)").unwrap();
    cleaned = empty_links_regex.replace_all(&cleaned, "").to_string();
    
    // Remove links with duplicate text in alt (like "SVG ImageSVG Image")
    // Since Rust regex doesn't support backreferences, we'll use a different approach
    // Match links and check for duplicates manually
    let link_regex = Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap();
    cleaned = link_regex
        .replace_all(&cleaned, |caps: &regex::Captures| {
            let text = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            // Check if text contains duplicate words/phrases (simple heuristic)
            // Split by common separators and check for duplicates
            let words: Vec<&str> = text.split_whitespace().collect();
            let mut has_duplicate = false;
            
            // Check if any word appears multiple times consecutively
            for i in 0..words.len().saturating_sub(1) {
                if words[i] == words[i + 1] && words[i].len() > 1 {
                    has_duplicate = true;
                    break;
                }
            }
            
            // Also check for patterns like "SVG ImageSVG Image" (no space between duplicates)
            if text.len() > 10 {
                let mid = text.len() / 2;
                let first_half = &text[..mid.min(text.len())];
                let second_half = &text[mid.min(text.len())..];
                if first_half == second_half && first_half.len() > 3 {
                    has_duplicate = true;
                }
            }
            
            // Check for navigation-related text
            let nav_keywords = ["SVG Image", "Navigate", "back to", "homepage"];
            if nav_keywords.iter().any(|&keyword| text.contains(keyword)) {
                has_duplicate = true;
            }
            
            if has_duplicate {
                String::new()
            } else {
                caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
            }
        })
        .to_string();
    
    // Remove standalone brackets that don't form valid markdown
    // Matches: [text] without (url) or (url) without [text] on the same line
    let orphan_brackets_regex = Regex::new(r"(?m)^\[[^\]]+\]$|^\([^)]+\)$").unwrap();
    cleaned = orphan_brackets_regex.replace_all(&cleaned, "").to_string();
    
    // Clean up multiple consecutive newlines (more than 2)
    let multiple_newlines_regex = Regex::new(r"\n{3,}").unwrap();
    cleaned = multiple_newlines_regex.replace_all(&cleaned, "\n\n").to_string();
    
    // Remove lines that are just brackets, parentheses, or whitespace
    let empty_bracket_lines_regex = Regex::new(r"^\s*[\[\]()]+\s*$").unwrap();
    let lines: Vec<String> = cleaned
        .lines()
        .filter(|line| !empty_bracket_lines_regex.is_match(line))
        .map(|s| s.to_string())
        .collect();
    cleaned = lines.join("\n");
    
    // Remove trailing/leading whitespace from each line and filter empty lines
    let trimmed_lines: Vec<String> = cleaned
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|s| s.to_string())
        .collect();
    cleaned = trimmed_lines.join("\n");
    
    cleaned
}

/// Comprehensive markdown cleaning function that applies all cleaning operations
pub fn clean_markdown(markdown: &str) -> String {
    let mut cleaned = markdown.to_string();
    
    // First strip data URI images
    cleaned = strip_data_uri_images(&cleaned);
    
    // Then clean up artifacts
    cleaned = clean_markdown_artifacts(&cleaned);
    
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_data_uri_images() {
        let markdown = r#"
# Test Document

This is some text.

![SVG Image](data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjwvc3ZnPg==)

More text here.

![PNG Image](data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==)

Even more text.
"#;
        
        let cleaned = strip_data_uri_images(markdown);
        
        // Should not contain data URI images
        assert!(!cleaned.contains("data:image/svg+xml"));
        assert!(!cleaned.contains("data:image/png"));
        assert!(cleaned.contains("This is some text"));
        assert!(cleaned.contains("More text here"));
        assert!(cleaned.contains("Even more text"));
    }

    #[test]
    fn test_strip_data_uri_images_preserves_regular_images() {
        let markdown = r#"
# Test Document

![Regular Image](https://example.com/image.png)

![Data URI Image](data:image/png;base64,abc123)

![Another Regular](/path/to/image.jpg)
"#;
        
        let cleaned = strip_data_uri_images(markdown);
        
        // Should preserve regular images
        assert!(cleaned.contains("![Regular Image](https://example.com/image.png)"));
        assert!(cleaned.contains("![Another Regular](/path/to/image.jpg)"));
        
        // Should remove data URI images
        assert!(!cleaned.contains("data:image/png"));
    }

    #[test]
    fn test_clean_markdown_artifacts() {
        let markdown = r#"
adjust [SVG ImageSVG Image Navigate back to the homepage](/ "Navigate back to the homepage")

Spaceout

beyond excelsior

Software development, application and system architecture

[about me](/about)[Contact](/contact)

[[[[[[[
"#;
        
        let cleaned = clean_markdown_artifacts(markdown);
        
        // Should remove duplicate text links
        assert!(!cleaned.contains("SVG ImageSVG Image"));
        assert!(!cleaned.contains("Navigate back to the homepage"));
        
        // Should remove multiple brackets
        assert!(!cleaned.contains("[[[[[[["));
        
        // Should preserve valid content
        assert!(cleaned.contains("Spaceout"));
        assert!(cleaned.contains("beyond excelsior"));
        assert!(cleaned.contains("Software development"));
    }

    #[test]
    fn test_clean_markdown_comprehensive() {
        let markdown = r#"
# Test

![SVG](data:image/svg+xml;base64,abc123)

adjust [SVG ImageSVG Image Navigate back](/ "Navigate")

[[[[[[[

[about me](/about)
"#;
        
        let cleaned = clean_markdown(markdown);
        
        // Should remove data URI images
        assert!(!cleaned.contains("data:image/svg+xml"));
        
        // Should remove duplicate text and navigation artifacts
        assert!(!cleaned.contains("SVG ImageSVG Image"));
        assert!(!cleaned.contains("Navigate back"));
        
        // Should remove multiple brackets
        assert!(!cleaned.contains("[[[[[[["));
        
        // Should preserve valid links
        assert!(cleaned.contains("[about me](/about)"));
    }
}

