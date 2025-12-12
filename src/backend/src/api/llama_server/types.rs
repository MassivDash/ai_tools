use std::collections::VecDeque;
use std::process::Child;
use std::sync::{Arc, Mutex};

pub type ProcessHandle = Arc<Mutex<Option<Child>>>;

#[derive(Clone, Debug)]
pub struct Config {
    pub hf_model: String,
    pub ctx_size: u32,
    // Advanced options
    pub threads: Option<i32>,
    pub threads_batch: Option<i32>,
    pub predict: Option<i32>,
    pub batch_size: Option<u32>,
    pub ubatch_size: Option<u32>,
    pub flash_attn: Option<bool>,
    pub mlock: Option<bool>,
    pub no_mmap: Option<bool>,
    pub gpu_layers: Option<u32>,
    pub model: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hf_model: "unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL".to_string(),
            ctx_size: 10240,
            threads: None,
            threads_batch: None,
            predict: None,
            batch_size: None,
            ubatch_size: None,
            flash_attn: None,
            mlock: None,
            no_mmap: None,
            gpu_layers: None,
            model: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: u64,
    pub line: String,
    pub source: LogSource,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LogSource {
    Stdout,
    Stderr,
}

pub type LogBuffer = Arc<Mutex<VecDeque<LogEntry>>>;

#[derive(Clone, Debug)]
pub struct ServerState {
    pub is_ready: bool,
}

pub type ServerStateHandle = Arc<Mutex<ServerState>>;
