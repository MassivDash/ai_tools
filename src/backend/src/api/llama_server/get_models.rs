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
}

#[derive(Serialize, Debug)]
pub struct ModelsResponse {
    pub local_models: Vec<ModelInfo>,
}

#[get("/api/llama-server/models")]
pub async fn get_llama_models() -> ActixResult<HttpResponse> {
    let cache_dir = match std::env::var("HOME") {
        Ok(home) => PathBuf::from(home).join(".cache").join("llama.cpp"),
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Could not determine home directory"
            })));
        }
    };

    let mut models = Vec::new();

    if cache_dir.exists() {
        println!("📂 Scanning for GGUF models in: {:?}", cache_dir);
        match scan_directory_for_gguf(&cache_dir, &mut models) {
            Ok(_) => {
                println!("✅ Found {} GGUF models", models.len());
            }
            Err(e) => {
                println!("⚠️  Error scanning directory: {}", e);
            }
        }
    } else {
        println!("⚠️  Cache directory does not exist: {:?}", cache_dir);
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

                        // Try to convert file name to HuggingFace format
                        let hf_format = parse_gguf_to_hf_format(&file_name);

                        models.push(ModelInfo {
                            name: file_name.clone(),
                            path: path.to_string_lossy().to_string(),
                            size,
                            hf_format,
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
        // Check if the pattern appears in the filename (might have dash or underscore before it)
        // Pattern could be: -Q6_K_XL or _Q6_K_XL or Q6_K_XL
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

    // Split by underscores to get parts
    let parts: Vec<&str> = name_without_ext.split('_').collect();

    if parts.len() < 2 {
        return None;
    }

    // First part is typically the user/org
    let user = parts[0];

    // Model name is typically the second part, but might include more
    // We'll try to find where the model name ends and quantization begins
    // Common pattern: user_model_base_model_variant_quant
    // We want: user/model_base:quant

    // If we found quantization, include it in the result
    if let Some(quant_str) = &quant {
        if parts.len() >= 2 {
            // Use second part as model name (first part is user)
            let model_name = parts[1];

            // Reconstruct as user/model:quant
            let result = format!("{}/{}:{}", user, model_name, quant_str);
            println!(
                "🔍 Parsed GGUF: {} -> {} (quant found: {})",
                filename, result, quant_str
            );
            return Some(result);
        }
    }

    // Fallback: if we can't find quantization, try a simple heuristic
    // Assume format: user_model_...
    if parts.len() >= 2 {
        // Try to use first two parts as user/model
        let model_name = parts[1];
        let result = format!("{}/{}", user, model_name);
        println!("🔍 Parsed GGUF (no quant): {} -> {}", filename, result);
        return Some(result);
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
