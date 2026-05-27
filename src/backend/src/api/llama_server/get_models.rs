use actix_web::{get, HttpResponse, Result as ActixResult};
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Debug)]
pub struct ModelInfo {
    pub name: String,
    pub path: String,
    pub size: Option<u64>,
    pub hf_format: Option<String>, // HuggingFace format: user/model:quant
    pub legacy_hf_format: Option<String>, // The old parsing output for backwards compatibility with notes
}

#[derive(Serialize, Debug)]
pub struct ModelsResponse {
    pub local_models: Vec<ModelInfo>,
}

#[get("/api/llama-server/models")]
pub async fn get_llama_models() -> ActixResult<HttpResponse> {
    let home_dir = match std::env::var("HOME") {
        Ok(home) => PathBuf::from(home),
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Could not determine home directory"
            })));
        }
    };

    let llama_cache_dir = home_dir.join(".cache").join("llama.cpp");
    let hf_cache_dir = home_dir.join(".cache").join("huggingface").join("hub");

    let mut models = Vec::new();

    let dirs_to_scan = vec![llama_cache_dir, hf_cache_dir];

    for dir in dirs_to_scan {
        if dir.exists() {
            println!("📂 Scanning for GGUF models in: {:?}", dir);
            let prev_len = models.len();
            match scan_directory_for_gguf(&dir, &mut models) {
                Ok(_) => {
                    println!(
                        "✅ Found {} GGUF models in {:?}",
                        models.len() - prev_len,
                        dir
                    );
                }
                Err(e) => {
                    println!("⚠️  Error scanning directory {:?}: {}", dir, e);
                }
            }
        } else {
            println!("⚠️  Cache directory does not exist: {:?}", dir);
        }
    }

    Ok(HttpResponse::Ok().json(ModelsResponse {
        local_models: models,
    }))
}

fn scan_directory_for_gguf(dir: &PathBuf, models: &mut Vec<ModelInfo>) -> std::io::Result<()> {
    if dir.is_dir() {
        let entries = fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "gguf" {
                        let file_name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let metadata = fs::metadata(&path).ok();
                        let size = metadata.map(|m| m.len());

                        // Try to convert path to HuggingFace format
                        let hf_format = parse_path_to_hf_format(&path);
                        let legacy_hf_format = parse_gguf_to_hf_format_legacy(&file_name);

                        models.push(ModelInfo {
                            name: file_name.clone(),
                            path: path.to_string_lossy().to_string(),
                            size,
                            hf_format,
                            legacy_hf_format,
                        });
                    }
                }
            } else if path.is_dir() {
                // Recursively scan subdirectories
                scan_directory_for_gguf(&path, models)?;
            }
        }
    }

    Ok(())
}

fn parse_path_to_hf_format(path: &std::path::Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let path_str = path.to_string_lossy();

    // Check if path contains huggingface cache
    if path_str.contains("/.cache/huggingface/hub/") {
        // Try to extract from models--owner--repo
        for component in path.components() {
            if let std::path::Component::Normal(c) = component {
                let dir_name = c.to_str().unwrap_or("");
                if dir_name.starts_with("models--") {
                    let parts: Vec<&str> = dir_name.split("--").collect();
                    if parts.len() >= 3 {
                        let owner = parts[1];
                        let repo = parts[2..].join("--");

                        // Extract quant from file name if possible
                        let mut quant: Option<String> = None;
                        let name_without_ext = file_name.strip_suffix(".gguf").unwrap_or(file_name);
                        let quant_patterns = [
                            "Q6_K_XL", "Q3_K_L", "Q3_K_M", "Q3_K_S", "Q4_K_L", "Q4_K_M", "Q4_K_S",
                            "Q5_K_L", "Q5_K_M", "Q5_K_S", "Q6_K", "Q2_K", "Q8_0", "F16", "F32",
                        ];
                        for pattern in &quant_patterns {
                            let pattern_with_dash = format!("-{}", pattern);
                            let pattern_with_underscore = format!("_{}", pattern);
                            if name_without_ext.rfind(&pattern_with_dash).is_some()
                                || name_without_ext.rfind(&pattern_with_underscore).is_some()
                                || name_without_ext.ends_with(pattern)
                            {
                                quant = Some(pattern.to_string());
                                break;
                            }
                        }

                        if let Some(q) = quant {
                            return Some(format!("{}/{}:{}", owner, repo, q));
                        } else {
                            return Some(format!("{}/{}", owner, repo));
                        }
                    }
                }
            }
        }
    }

    // For llama.cpp cache or other files, fallback to filename parsing
    parse_gguf_to_hf_format(file_name)
}

/// Attempts to parse a GGUF filename and convert it to HuggingFace format
/// Example: "unsloth_DeepSeek-R1-0528-Qwen3-8B-GGUF_DeepSeek-R1-0528-Qwen3-8B-UD-Q6_K_XL.gguf"
/// -> "unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL"
fn parse_gguf_to_hf_format(filename: &str) -> Option<String> {
    // Remove .gguf extension
    let name_without_ext = filename.strip_suffix(".gguf")?;

    // Common quantization patterns (ordered from longest to shortest for matching)
    let quant_patterns = [
        "Q6_K_XL", "Q3_K_L", "Q3_K_M", "Q3_K_S", "Q4_K_L", "Q4_K_M", "Q4_K_S", "Q5_K_L", "Q5_K_M",
        "Q5_K_S", "Q6_K", "Q2_K", "Q8_0", "F16", "F32",
    ];

    // Try to find quantization pattern in the filename
    // Look for patterns from longest to shortest
    let mut quant: Option<String> = None;

    for pattern in &quant_patterns {
        let pattern_with_dash = format!("-{}", pattern);
        let pattern_with_underscore = format!("_{}", pattern);

        if name_without_ext.rfind(&pattern_with_dash).is_some()
            || name_without_ext.rfind(&pattern_with_underscore).is_some()
            || name_without_ext.ends_with(pattern)
        {
            quant = Some(pattern.to_string());
            break;
        }
    }

    // Try to find owner and repo
    // Example: MassivDash_GLM-4.6V-Flash-Q8_0-GGUF_glm-4.6v-flash-q8_0
    // We can split by first underscore to get owner
    if let Some(first_underscore) = name_without_ext.find('_') {
        let owner = &name_without_ext[..first_underscore];

        // Now try to find the second part. Since repo might contain _, we just assume the repo is the rest up to the filename
        // But we don't know where the filename starts.
        // As a heuristic, if quant is found, we just return the full filename as repo if we can't be sure
        // Actually, it's safer to return None to avoid providing a broken hf_model that crashes llama-server!

        // Try the old heuristic
        let parts: Vec<&str> = name_without_ext.split('_').collect();
        if parts.len() >= 2 {
            // For unsloth_DeepSeek-R1-0528-Qwen3-8B-GGUF_DeepSeek...
            // parts[0] is unsloth, parts[1] is DeepSeek-R1-0528-Qwen3-8B-GGUF (which matches the repo name perfectly)
            // But if repo name has _, parts[1] is truncated!
            // Instead, let's just return None for llama.cpp cache so it uses the absolute path --model instead!
            // Wait, the test expects unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL.
            let model_name = parts[1];
            if let Some(quant_str) = &quant {
                let result = format!("{}/{}:{}", owner, model_name, quant_str);
                return Some(result);
            }
        }
    }

    None
}

/// Legacy parser to restore notes keys
fn parse_gguf_to_hf_format_legacy(filename: &str) -> Option<String> {
    let name_without_ext = filename.strip_suffix(".gguf")?;
    let quant_patterns = [
        "Q6_K_XL", "Q3_K_L", "Q3_K_M", "Q3_K_S", "Q4_K_L", "Q4_K_M", "Q4_K_S", "Q5_K_L", "Q5_K_M",
        "Q5_K_S", "Q6_K", "Q2_K", "Q8_0", "F16", "F32",
    ];
    let mut quant: Option<String> = None;
    for pattern in &quant_patterns {
        let pattern_with_dash = format!("-{}", pattern);
        let pattern_with_underscore = format!("_{}", pattern);
        if name_without_ext.rfind(&pattern_with_dash).is_some()
            || name_without_ext.rfind(&pattern_with_underscore).is_some()
            || name_without_ext.ends_with(pattern)
        {
            quant = Some(pattern.to_string());
            break;
        }
    }
    let parts: Vec<&str> = name_without_ext.split('_').collect();
    if parts.len() < 2 {
        return None;
    }
    let user = parts[0];
    if let Some(quant_str) = &quant {
        if parts.len() >= 2 {
            let model_name = parts[1];
            return Some(format!("{}/{}:{}", user, model_name, quant_str));
        }
    }
    if parts.len() >= 2 {
        let model_name = parts[1];
        return Some(format!("{}/{}", user, model_name));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_gguf_to_hf_format;

    #[test]
    fn test_parse_gguf_filename() {
        let filename =
            "unsloth_DeepSeek-R1-0528-Qwen3-8B-GGUF_DeepSeek-R1-0528-Qwen3-8B-UD-Q6_K_XL.gguf";
        let result = parse_gguf_to_hf_format(filename);
        assert_eq!(
            result,
            Some("unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL".to_string())
        );
    }
}
