use std::collections::VecDeque;
use std::process::Child;
use std::sync::{Arc, Mutex};

pub type ProcessHandle = Arc<Mutex<Option<Child>>>;

#[derive(Clone, Debug)]
pub struct Config {
    pub hf_model: String,
    pub ctx_size: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hf_model: "unsloth/DeepSeek-R1-0528-Qwen3-8B-GGUF:Q6_K_XL".to_string(),
            ctx_size: 10240,
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
