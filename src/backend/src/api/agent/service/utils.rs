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
