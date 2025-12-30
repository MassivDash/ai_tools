use crate::api::chromadb::config::types::{ModelInfo, ModelsResponse};
use actix_web::{get, HttpResponse, Result as ActixResult};
use std::process::Command;

#[get("/api/chromadb/models")]
pub async fn get_ollama_models() -> ActixResult<HttpResponse> {
    println!("📋 Fetching Ollama models using 'ollama list' command...");

    // Execute ollama list command
    let output =
        match tokio::task::spawn_blocking(|| Command::new("ollama").arg("list").output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                println!("Failed to execute ollama list: {}", e);
                return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to execute ollama list: {}", e)
                })));
            }
            Err(e) => {
                println!("Failed to spawn ollama list task: {}", e);
                return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Failed to spawn task: {}", e)
                })));
            }
        };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("ollama list command failed: {}", stderr);
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("ollama list command failed: {}", stderr)
        })));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let models = parse_ollama_list_output(&stdout);

    println!("✅ Found {} Ollama models", models.len());

    Ok(HttpResponse::Ok().json(ModelsResponse { models }))
}

/// Parse the output of `ollama list` command
/// Expected format:
/// NAME                    ID              SIZE    MODIFIED
/// llama3.2:latest         abc123def456    4.7 GB  2 hours ago
/// mistral:latest          def456ghi789    3.2 GB  1 day ago
fn parse_ollama_list_output(output: &str) -> Vec<ModelInfo> {
    let mut models = Vec::new();
    let lines: Vec<&str> = output.lines().collect();

    // Skip header line (first line)
    if lines.len() < 2 {
        return models;
    }

    for line in lines.iter().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Split by whitespace, but handle the fact that model names might contain spaces
        // The format is: NAME ID SIZE MODIFIED
        // We'll split and take the first 4 parts
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() >= 2 {
            let name = parts[0].to_string();
            // Skip ID (parts[1]) as we don't need it
            let size = parts.get(2).map(|s| {
                // Combine size parts if there are multiple (e.g., "4.7 GB")
                if parts.len() > 3 {
                    format!("{} {}", s, parts.get(3).unwrap_or(&""))
                } else {
                    s.to_string()
                }
            });
            let modified = if parts.len() > 4 {
                // Modified date might be multiple words (e.g., "2 hours ago")
                Some(parts[4..].join(" "))
            } else {
                None
            };

            models.push(ModelInfo {
                name,
                size,
                modified,
            });
        }
    }

    models
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ollama_list_output() {
        let output = "NAME                    ID              SIZE    MODIFIED\nllama3.2:latest         abc123def456    4.7 GB  2 hours ago\nmistral:latest          def456ghi789    3.2 GB  1 day ago";
        let models = parse_ollama_list_output(output);

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "llama3.2:latest");
        assert_eq!(models[0].size, Some("4.7 GB".to_string()));
        assert_eq!(models[1].name, "mistral:latest");
    }
}
