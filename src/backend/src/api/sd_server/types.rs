use std::collections::VecDeque;
use std::process::Child;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum LogSource {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: u64,
    pub line: String,
    pub source: LogSource,
}

pub type LogBuffer = Arc<Mutex<VecDeque<LogEntry>>>;
pub type SDProcessHandle = Arc<Mutex<Option<Child>>>;
pub type SDConfigHandle = Arc<Mutex<SDConfig>>;

#[derive(Clone, Debug)]
pub struct SDState {
    pub is_generating: bool,
    pub current_output_file: Option<String>,
    pub pending_filename: Option<String>,
}

pub type SDStateHandle = Arc<Mutex<SDState>>;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SDConfig {
    // CLI Options
    pub output_path: String,
    pub preview_path: Option<String>,
    pub preview_interval: Option<u32>,
    pub output_begin_idx: Option<u32>,
    pub canny: bool,
    pub verbose: bool,
    pub color: bool,
    pub mode: Option<String>,           // img_gen, vid_gen, etc.
    pub preview_method: Option<String>, // none, proj, tae, vae

    // Context Options
    pub diffusion_model: String,
    pub model: Option<String>, // path to full model
    pub clip_l: Option<String>,
    pub clip_g: Option<String>,
    pub clip_vision: Option<String>,
    pub t5xxl: Option<String>,
    pub llm: Option<String>,
    pub llm_vision: Option<String>,
    pub vae: Option<String>,
    pub taesd: Option<String>,
    pub control_net: Option<String>,
    pub embd_dir: Option<String>,
    pub lora_model_dir: Option<String>,
    pub upscale_model: Option<String>,
    pub threads: i32,
    pub offload_to_cpu: bool,
    pub diffusion_fa: bool,
    pub control_net_cpu: bool,
    pub clip_on_cpu: bool,
    pub vae_on_cpu: bool,
    pub vae_tiling: bool,
    pub vae_tile_size: Option<u32>,
    pub vae_relative_tile_size: Option<f32>,
    pub models_path: String,
    pub rng: String, // cuda, cpu, std_default

    // Generation Options
    pub prompt: String,
    pub negative_prompt: String,
    pub init_img: Option<String>,
    pub mask: Option<String>,
    pub control_image: Option<String>,
    pub height: u32,
    pub width: u32,
    pub steps: Option<u32>,
    pub batch_count: Option<u32>,
    pub cfg_scale: f32,
    pub guidance: Option<f32>,
    pub strength: Option<f32>,
    pub seed: Option<i64>,
    pub sampling_method: Option<String>,
    pub scheduler: Option<String>,
}

impl Default for SDConfig {
    fn default() -> Self {
        Self {
            output_path: "./public".to_string(),
            preview_path: None,
            preview_interval: None,
            output_begin_idx: None,
            canny: false,
            verbose: true,
            color: true,
            mode: None, // Default None
            preview_method: None,

            diffusion_model: "z_image_turbo-Q8_0.gguf".to_string(),
            model: None,
            clip_l: None,
            clip_g: None,
            clip_vision: None,
            t5xxl: None,
            llm: Some("Qwen3-4B-Instruct-2507-Q8_0.gguf".to_string()),
            llm_vision: None,
            vae: Some("ae.safetensors".to_string()),
            taesd: None,
            control_net: None,
            embd_dir: None,
            lora_model_dir: None,
            upscale_model: None,
            threads: -1,
            offload_to_cpu: true,
            diffusion_fa: true,
            models_path: "./sd_models".to_string(),
            rng: "std_default".to_string(),

            // Low VRAM defaults
            control_net_cpu: true,
            clip_on_cpu: true,
            vae_on_cpu: true,
            vae_tiling: true,
            vae_tile_size: None,          // Optional override
            vae_relative_tile_size: None, // Optional override

            prompt: "A beautiful landscape".to_string(),
            negative_prompt: "".to_string(),
            init_img: None,
            mask: None,
            control_image: None,
            height: 1024,
            width: 1024,
            steps: None,       // Default None
            batch_count: None, // Default None
            cfg_scale: 1.0,
            guidance: None,        // Default None
            strength: None,        // Default None
            seed: None,            // Default None
            sampling_method: None, // Default None
            scheduler: None,       // Default None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sd_config_default_cli_and_context_options() {
        let config = SDConfig::default();

        assert_eq!(config.output_path, "./public");
        assert!(config.preview_path.is_none());
        assert!(config.preview_interval.is_none());
        assert!(config.output_begin_idx.is_none());
        assert!(!config.canny);
        assert!(config.verbose);
        assert!(config.color);
        assert!(config.mode.is_none());
        assert!(config.preview_method.is_none());

        assert_eq!(config.diffusion_model, "z_image_turbo-Q8_0.gguf");
        assert!(config.model.is_none());
        assert_eq!(
            config.llm.as_deref(),
            Some("Qwen3-4B-Instruct-2507-Q8_0.gguf")
        );
        assert_eq!(config.vae.as_deref(), Some("ae.safetensors"));
        assert_eq!(config.threads, -1);
        assert_eq!(config.models_path, "./sd_models");
        assert_eq!(config.rng, "std_default");
    }

    #[test]
    fn test_sd_config_default_is_tuned_for_low_vram() {
        let config = SDConfig::default();

        assert!(config.offload_to_cpu);
        assert!(config.diffusion_fa);
        assert!(config.control_net_cpu);
        assert!(config.clip_on_cpu);
        assert!(config.vae_on_cpu);
        assert!(config.vae_tiling);
        // Tile sizes are left to sd.cpp unless explicitly overridden.
        assert!(config.vae_tile_size.is_none());
        assert!(config.vae_relative_tile_size.is_none());
    }

    #[test]
    fn test_sd_config_default_generation_options() {
        let config = SDConfig::default();

        assert_eq!(config.prompt, "A beautiful landscape");
        assert_eq!(config.negative_prompt, "");
        assert_eq!(config.width, 1024);
        assert_eq!(config.height, 1024);
        assert_eq!(config.cfg_scale, 1.0);
        // Everything the CLI has its own default for stays unset.
        assert!(config.steps.is_none());
        assert!(config.batch_count.is_none());
        assert!(config.guidance.is_none());
        assert!(config.strength.is_none());
        assert!(config.seed.is_none());
        assert!(config.sampling_method.is_none());
        assert!(config.scheduler.is_none());
        assert!(config.init_img.is_none());
        assert!(config.mask.is_none());
        assert!(config.control_image.is_none());
    }

    #[test]
    fn test_sd_config_survives_a_json_round_trip() {
        let config = SDConfig {
            prompt: "a fox in snow".to_string(),
            steps: Some(28),
            seed: Some(-1),
            cfg_scale: 4.5,
            vae_relative_tile_size: Some(0.25),
            ..SDConfig::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        let restored: SDConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.prompt, "a fox in snow");
        assert_eq!(restored.steps, Some(28));
        assert_eq!(restored.seed, Some(-1));
        assert_eq!(restored.cfg_scale, 4.5);
        assert_eq!(restored.vae_relative_tile_size, Some(0.25));
        assert_eq!(restored.diffusion_model, config.diffusion_model);
    }

    #[test]
    fn test_log_source_equality() {
        assert_eq!(LogSource::Stdout, LogSource::Stdout);
        assert_ne!(LogSource::Stdout, LogSource::Stderr);
    }

    #[test]
    fn test_log_entry_and_state_are_cloneable_snapshots() {
        let entry = LogEntry {
            timestamp: 1700000000,
            line: "loading model".to_string(),
            source: LogSource::Stderr,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.timestamp, 1700000000);
        assert_eq!(cloned.line, "loading model");
        assert_eq!(cloned.source, LogSource::Stderr);

        let state = SDState {
            is_generating: true,
            current_output_file: Some("/public/a.png".to_string()),
            pending_filename: Some("a.png".to_string()),
        };
        let cloned_state = state.clone();
        assert!(cloned_state.is_generating);
        assert_eq!(
            cloned_state.current_output_file.as_deref(),
            Some("/public/a.png")
        );
        assert_eq!(cloned_state.pending_filename.as_deref(), Some("a.png"));
    }

    #[test]
    fn test_shared_handles_alias_the_same_state() {
        let buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        let state: SDStateHandle = Arc::new(Mutex::new(SDState {
            is_generating: false,
            current_output_file: None,
            pending_filename: None,
        }));

        let buffer_clone = buffer.clone();
        let state_clone = state.clone();

        buffer_clone.lock().unwrap().push_back(LogEntry {
            timestamp: 1,
            line: "x".to_string(),
            source: LogSource::Stdout,
        });
        state_clone.lock().unwrap().is_generating = true;

        assert_eq!(buffer.lock().unwrap().len(), 1);
        assert!(state.lock().unwrap().is_generating);
    }
}
