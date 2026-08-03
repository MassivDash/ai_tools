/// Clean response text by removing internal reasoning markers and redacted content
pub fn clean_response(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Remove redacted reasoning markers
    cleaned = cleaned.replace("<|redacted_reasoning|>", "");
    cleaned = cleaned.replace("</think>", "");
    cleaned = cleaned.replace("<think>", "");
    cleaned = cleaned.replace("</think>", "");

    // Remove tool call markers
    cleaned = cleaned.replace("<｜tool▁calls▁begin｜>", "");
    cleaned = cleaned.replace("<｜tool▁calls▁end｜>", "");
    cleaned = cleaned.replace("<｜tool▁call▁begin｜>", "");
    cleaned = cleaned.replace("<｜tool▁call▁end｜>", "");
    cleaned = cleaned.replace("<｜tool▁sep｜>", "");
    cleaned = cleaned.replace("<｜tool▁outputs▁begin｜>", "");
    cleaned = cleaned.replace("<｜tool▁outputs▁end｜>", "");
    cleaned = cleaned.replace("<｜tool▁output▁begin｜>", "");
    cleaned = cleaned.replace("<｜tool▁output▁end｜>", "");

    // Remove common internal reasoning patterns (Thought/Action/Observation format)
    if cleaned.contains("Thought:")
        || cleaned.contains("Action:")
        || cleaned.contains("Observation:")
    {
        // Try to extract just the answer if present
        if let Some(answer_start) = cleaned.rfind("Answer:") {
            cleaned = cleaned[answer_start + 7..].trim().to_string();
        } else if let Some(answer_start) = cleaned.rfind("answer:") {
            cleaned = cleaned[answer_start + 7..].trim().to_string();
        } else {
            // If no Answer found, try to remove the reasoning blocks
            // Look for patterns like "Thought: ... Action: ... Observation: ..."
            let lines: Vec<&str> = cleaned.lines().collect();
            let mut filtered_lines = Vec::new();
            let mut skip_until_answer = false;

            for line in lines {
                let line_lower = line.trim().to_lowercase();
                if line_lower.starts_with("thought:")
                    || line_lower.starts_with("action:")
                    || line_lower.starts_with("observation:")
                    || line_lower.starts_with("current task:")
                    || line_lower.starts_with("you are in a new chain")
                {
                    skip_until_answer = true;
                    continue;
                }
                if line_lower.starts_with("answer:") {
                    skip_until_answer = false;
                    filtered_lines.push(&line[7..]); // Skip "Answer:" prefix
                    continue;
                }
                if !skip_until_answer {
                    filtered_lines.push(line);
                }
            }
            if !filtered_lines.is_empty() {
                cleaned = filtered_lines.join("\n");
            }
        }
    }

    // Remove any remaining HTML-like tags that might be internal markers
    // Use simple string replacement instead of regex for reliability
    let mut result = String::new();
    let mut in_tag = false;
    for ch in cleaned.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' && in_tag {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    cleaned = result;

    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text_is_only_trimmed() {
        assert_eq!(clean_response("  Hello world  "), "Hello world");
        assert_eq!(clean_response(""), "");
    }

    #[test]
    fn test_think_and_redacted_reasoning_markers_are_removed() {
        assert_eq!(
            clean_response("<think>internal</think>The answer"),
            "internalThe answer"
        );
        assert_eq!(clean_response("<|redacted_reasoning|>visible"), "visible");
    }

    #[test]
    fn test_deepseek_tool_markers_are_removed() {
        let raw = "<｜tool▁calls▁begin｜><｜tool▁call▁begin｜>weather<｜tool▁sep｜>args\
                   <｜tool▁call▁end｜><｜tool▁calls▁end｜>\
                   <｜tool▁outputs▁begin｜><｜tool▁output▁begin｜>sunny\
                   <｜tool▁output▁end｜><｜tool▁outputs▁end｜>";

        assert_eq!(clean_response(raw), "weatherargssunny");
    }

    #[test]
    fn test_answer_marker_keeps_only_the_final_answer() {
        assert_eq!(
            clean_response("Thought: I should check the weather\nAnswer: It is sunny"),
            "It is sunny"
        );
        // The *last* Answer: wins
        assert_eq!(
            clean_response("Answer: first\nAction: more work\nAnswer: second"),
            "second"
        );
        // Lowercase spelling is handled too
        assert_eq!(
            clean_response("Observation: it rained\nanswer: bring an umbrella"),
            "bring an umbrella"
        );
    }

    #[test]
    fn test_reasoning_lines_are_dropped_when_no_answer_marker_exists() {
        let raw = "Here is the summary.\nThought: I should double check\nAction: search\nObservation: found it";

        assert_eq!(clean_response(raw), "Here is the summary.");
    }

    #[test]
    fn test_scaffolding_prefixes_are_dropped() {
        let raw = "Result line\nCurrent task: do something\nYou are in a new chain\nThought: hmm";

        assert_eq!(clean_response(raw), "Result line");
    }

    #[test]
    fn test_uppercase_answer_line_resumes_output_after_reasoning() {
        // "Answer:"/"answer:" are absent, so the line filter runs and the
        // case-insensitive "ANSWER:" line switches output back on.
        let raw = "Thought: hmm\nANSWER: forty two\nand some detail";

        assert_eq!(clean_response(raw), "forty two\nand some detail");
    }

    #[test]
    fn test_reasoning_only_text_is_left_untouched() {
        // Every line is filtered out, so the function falls back to the original
        // text rather than returning an empty string.
        let raw = "Thought: hmm\nAction: search";

        assert_eq!(clean_response(raw), raw);
    }

    #[test]
    fn test_remaining_html_like_tags_are_stripped() {
        assert_eq!(clean_response("a <b>c</b> d"), "a c d");
        assert_eq!(clean_response("unclosed <tag"), "unclosed");
        assert_eq!(clean_response("stray > angle"), "stray > angle");
    }
}
