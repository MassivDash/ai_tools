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

#[derive(Clone, Debug)]
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
            offload_to_cpu: false,
            diffusion_fa: true,
            models_path: "./sd_models".to_string(),
            rng: "std_default".to_string(),

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
