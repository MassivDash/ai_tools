use std::ops::Range;

/// Some models leak the ask_human tool call as literal JSON in the text
/// content instead of using native tool-calling, especially on long
/// generations. Scans `text` for a balanced `{...}` object shaped like
/// ask_human's arguments (a "question" string and a non-empty "options"
/// array of strings) and returns it along with its byte range so the caller
/// can promote it to a real tool call and strip it from the visible text.
pub fn extract_leaked_ask_human_json(text: &str) -> Option<(serde_json::Value, Range<usize>)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some(end) = find_matching_brace(text, i) {
                let candidate = &text[i..=end];
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(candidate) {
                    if is_ask_human_shape(&value) {
                        return Some((value, i..end + 1));
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Removes the JSON object at `range` from `text`, along with an
/// immediately adjacent opening/closing markdown code fence (e.g.
/// ```json ... ```), if present.
pub fn strip_json_block(text: &str, range: Range<usize>) -> String {
    let mut start = range.start;
    let mut end = range.end;

    let prefix = &text[..start];
    if let Some(fence_start) = prefix.trim_end().rfind("```") {
        let between = &prefix[fence_start + 3..];
        if between.chars().all(|c| c.is_alphanumeric() || c.is_whitespace()) {
            start = fence_start;
        }
    }

    let suffix = &text[end..];
    let leading_ws = suffix.len() - suffix.trim_start().len();
    if suffix[leading_ws..].starts_with("```") {
        end += leading_ws + 3;
    }

    let mut result = String::with_capacity(text.len());
    result.push_str(&text[..start]);
    result.push_str(&text[end..]);
    result.trim().to_string()
}

fn is_ask_human_shape(value: &serde_json::Value) -> bool {
    let has_question = value.get("question").and_then(|q| q.as_str()).is_some();
    let has_options = value
        .get("options")
        .and_then(|o| o.as_array())
        .map(|arr| !arr.is_empty() && arr.iter().all(|item| item.is_string()))
        .unwrap_or(false);
    has_question && has_options
}

/// Finds the byte index of the `}` matching the `{` at `start`, respecting
/// string literals (so braces inside option text don't throw off the count).
fn find_matching_brace(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut idx = start;
    while idx < bytes.len() {
        let c = bytes[idx];
        if in_string {
            if escape {
                escape = false;
            } else if c == b'\\' {
                escape = true;
            } else if c == b'"' {
                in_string = false;
            }
        } else {
            match c {
                b'"' => in_string = true,
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(idx);
                    }
                }
                _ => {}
            }
        }
        idx += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_fenced_json_mid_message() {
        let text = "Sure, here are your options:\n\n```json\n{\"question\": \"Pick one\", \"options\": [\"A\", \"B\", \"Other\"]}\n```\n\nLet me know what you think!";
        let (value, range) = extract_leaked_ask_human_json(text).expect("should detect leaked json");
        assert_eq!(value["question"], "Pick one");
        let cleaned = strip_json_block(text, range);
        assert!(!cleaned.contains("```"));
        assert!(!cleaned.contains("question"));
        assert!(cleaned.contains("Sure, here are your options:"));
        assert!(cleaned.contains("Let me know what you think!"));
    }

    #[test]
    fn detects_bare_json_no_fence() {
        let text = "{\"question\": \"Continue?\", \"options\": [\"Yes\", \"No\", \"Other\"]}";
        let (value, _) = extract_leaked_ask_human_json(text).expect("should detect leaked json");
        assert_eq!(value["options"][0], "Yes");
    }

    #[test]
    fn ignores_unrelated_json() {
        let text = "Here's some config: {\"foo\": \"bar\", \"nested\": {\"a\": 1}}";
        assert!(extract_leaked_ask_human_json(text).is_none());
    }

    #[test]
    fn ignores_plain_text() {
        let text = "Just a normal answer with no JSON at all.";
        assert!(extract_leaked_ask_human_json(text).is_none());
    }
}
