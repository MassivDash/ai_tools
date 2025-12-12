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
                        let size = metadata.and_then(|m| Some(m.len()));

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
    
    // Split by underscores
    let parts: Vec<&str> = name_without_ext.split('_').collect();
    
    if parts.len() < 3 {
        // Not enough parts to parse
        return None;
    }
    
    // Common quantization patterns (ordered from longest to shortest)
    let quant_patterns = [
        "Q3_K_L", "Q3_K_M", "Q3_K_S", "Q4_K_L", "Q4_K_M", "Q4_K_S",
        "Q5_K_L", "Q5_K_M", "Q5_K_S", "Q6_K_XL", "Q6_K", "Q2_K", "Q8_0", "F16", "F32",
    ];
    
    // Try to find quantization by checking last 1-3 parts
    let mut quant: Option<String> = None;
    let mut model_parts_end = parts.len();
    
    // Check last 3 parts combined (for patterns like Q6_K_XL)
    if parts.len() >= 3 {
        let last_three = format!("{}_{}_{}", 
            parts[parts.len() - 3], 
            parts[parts.len() - 2], 
            parts[parts.len() - 1]);
        for pattern in &quant_patterns {
            if last_three.starts_with(pattern) || last_three == *pattern {
                quant = Some(last_three);
                model_parts_end = parts.len() - 3;
                break;
            }
        }
    }
    
    // Check last 2 parts combined (for patterns like Q3_K_M)
    if quant.is_none() && parts.len() >= 2 {
        let last_two = format!("{}_{}", parts[parts.len() - 2], parts[parts.len() - 1]);
        for pattern in &quant_patterns {
            if last_two.starts_with(pattern) || last_two == *pattern {
                quant = Some(last_two);
                model_parts_end = parts.len() - 2;
                break;
            }
        }
    }
    
    // Check last part alone (for patterns like Q8_0, F16, F32)
    if quant.is_none() {
        if let Some(last_part) = parts.last() {
            for pattern in &quant_patterns {
                if last_part.starts_with(pattern) || last_part == pattern {
                    quant = Some(last_part.to_string());
                    model_parts_end = parts.len() - 1;
                    break;
                }
            }
        }
    }
    
    // First part is typically the user/org
    let user = parts[0];
    
    // Model name is typically the second part, but might include more
    // We'll try to find where the model name ends and quantization begins
    // Common pattern: user_model_base_model_variant_quant
    // We want: user/model_base:quant
    
    // If we found quantization, model is typically just the second part
    // Pattern: user_model_base_model_variant_quant
    // We want: user/model_base:quant (ignore variant)
    if let Some(quant_str) = &quant {
        if model_parts_end > 1 {
            // Use second part as model name (first part is user)
            let model_name = parts[1];
            
            // Reconstruct as user/model:quant
            return Some(format!("{}:{}", format!("{}/{}", user, model_name), quant_str));
        }
    }
    
    // Fallback: if we can't parse, try a simple heuristic
    // Assume format: user_model_..._quant
    if parts.len() >= 2 {
        // Try to use first two parts as user/model
        let model_name = parts[1];
        if let Some(quant_str) = &quant {
            return Some(format!("{}:{}", format!("{}/{}", user, model_name), quant_str));
        } else {
            // No quant found, just return user/model
            return Some(format!("{}/{}", user, model_name));
        }
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::parse_gguf_to_hf_format;

    #[test]
    fn test_parse_gguf_filename() {
        let filename = "unsloth_DeepSeek-R1-0528-Qwen3-8B-GGUF_DeepSeek-R1-0528-Qwen3-8B-UD-Q6_K_XL.gguf";
        let result = parse_gguf_to_hf_format(filename);
        assert_eq!(
            result,
            Some("unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL".to_string())
        );
    }
}
