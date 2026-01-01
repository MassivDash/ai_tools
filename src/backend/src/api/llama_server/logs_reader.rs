use crate::api::llama_server::types::{LogBuffer, LogEntry, LogSource, ServerStateHandle};
use crate::api::llama_server::websocket::WebSocketState;
use std::io::{BufRead, BufReader};
use std::process::{ChildStderr, ChildStdout};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn spawn_log_reader(
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    log_buffer: LogBuffer,
    server_state: ServerStateHandle,
    ws_state: Option<Arc<WebSocketState>>,
    port: Option<u16>,
) {
    let current_generation = {
        let state = server_state.lock().unwrap();
        state.generation
    };

    if let Some(stdout_handle) = stdout {
        let log_buffer_clone = log_buffer.clone();
        let server_state_clone = server_state.clone();
        let ws_state_clone = ws_state.clone();
        std::thread::spawn(move || {
            read_stdout_stream(
                stdout_handle,
                log_buffer_clone,
                server_state_clone,
                ws_state_clone,
                current_generation,
                port,
            );
        });
    }

    if let Some(stderr_handle) = stderr {
        let log_buffer_clone = log_buffer.clone();
        let server_state_clone = server_state.clone();
        let ws_state_clone = ws_state.clone();
        std::thread::spawn(move || {
            read_stderr_stream(
                stderr_handle,
                log_buffer_clone,
                server_state_clone,
                ws_state_clone,
                current_generation,
                port,
            );
        });
    }
}

fn read_stdout_stream(
    stream: ChildStdout,
    log_buffer: LogBuffer,
    server_state: ServerStateHandle,
    ws_state: Option<Arc<WebSocketState>>,
    generation: u32,
    port: Option<u16>,
) {
    let reader = BufReader::new(stream);
    let lines = reader.lines();

    for line_result in lines {
        match line_result {
            Ok(line) => {
                process_log_line(
                    line,
                    log_buffer.clone(),
                    server_state.clone(),
                    LogSource::Stdout,
                    ws_state.clone(),
                    generation,
                    port,
                );
            }
            Err(e) => {
                eprintln!("Error reading stdout line: {}", e);
                break;
            }
        }
    }
}

fn read_stderr_stream(
    stream: ChildStderr,
    log_buffer: LogBuffer,
    server_state: ServerStateHandle,
    ws_state: Option<Arc<WebSocketState>>,
    generation: u32,
    port: Option<u16>,
) {
    let reader = BufReader::new(stream);
    let lines = reader.lines();

    for line_result in lines {
        match line_result {
            Ok(line) => {
                process_log_line(
                    line,
                    log_buffer.clone(),
                    server_state.clone(),
                    LogSource::Stderr,
                    ws_state.clone(),
                    generation,
                    port,
                );
            }
            Err(e) => {
                eprintln!("Error reading stderr line: {}", e);
                break;
            }
        }
    }
}

fn process_log_line(
    line: String,
    log_buffer: LogBuffer,
    server_state: ServerStateHandle,
    source: LogSource,
    ws_state: Option<Arc<WebSocketState>>,
    generation: u32,
    port: Option<u16>,
) {
    // Validate generation
    {
        let state = server_state.lock().unwrap();
        if state.generation != generation {
            // Stale log reader from previous process, ignore
            return;
        }
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let log_entry = LogEntry {
        timestamp,
        line: line.clone(),
        source: source.clone(),
    };

    // Add to log buffer (keep last 1000 lines)
    {
        let mut buffer = log_buffer.lock().unwrap();
        buffer.push_back(log_entry.clone());
        if buffer.len() > 1000 {
            buffer.pop_front();
        }
    }

    // Broadcast log via WebSocket
    if let Some(ref state) = ws_state {
        use crate::api::llama_server::websocket::LogLine;
        let log_line = LogLine {
            timestamp: log_entry.timestamp,
            line: log_entry.line.clone(),
            source: match log_entry.source {
                LogSource::Stdout => "stdout".to_string(),
                LogSource::Stderr => "stderr".to_string(),
            },
        };
        state.broadcast_log(log_line);
    } else {
        println!("⚠️  WebSocket state not available for broadcasting logs");
    }

    // Check if server is ready - Generalize check to support any port/host
    // We check for tokens individually to handle potential ANSI color codes in the output
    let is_ready_msg =
        (line.contains("main") && line.contains("listening") && line.contains("http"))
            || line.contains("HTTP server listening");

    if is_ready_msg {
        println!("✅ Detected server ready message in line: '{}'", line);
        let mut state = server_state.lock().unwrap();
        // Double check generation before setting ready
        if state.generation == generation {
            state.is_ready = true;
            drop(state);

            // Broadcast active status
            if let Some(ref state) = ws_state {
                let actual_port = port.unwrap_or(8080);
                println!(
                    "📡 Broadcasting server ready status on port {}",
                    actual_port
                );
                state.broadcast_status(true, actual_port);
            }
        } else {
            let msg = format!(
                "❌ SYSTEM: Generation mismatch ignoring ready signal: {} != {}",
                state.generation, generation
            );
            println!("{}", msg);
        }
    } else if line.contains("listening") {
        let msg = format!(
            "❓ SYSTEM: Line contains 'listening' but failed full check: '{}'",
            line
        );
        println!("{}", msg);
    }

    println!(
        "📝 [{}] {}",
        if source == LogSource::Stdout {
            "stdout"
        } else {
            "stderr"
        },
        line
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::llama_server::types::ServerState;
    use std::collections::VecDeque;

    #[test]
    fn test_process_log_line_readiness_plain() {
        let log_buffer: LogBuffer = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let server_state: ServerStateHandle = Arc::new(std::sync::Mutex::new(ServerState {
            is_ready: false,
            generation: 1,
        }));
        let line = "main: server is listening on http://0.0.0.0:8099".to_string();

        process_log_line(
            line,
            log_buffer,
            server_state.clone(),
            LogSource::Stdout,
            None, // No WebSocket for this test
            1,    // Generation match
            Some(8099),
        );

        let state = server_state.lock().unwrap();
        assert!(state.is_ready, "Server should be ready with plain log line");
    }

    #[test]
    fn test_process_log_line_readiness_ansi() {
        let log_buffer: LogBuffer = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let server_state: ServerStateHandle = Arc::new(std::sync::Mutex::new(ServerState {
            is_ready: false,
            generation: 1,
        }));
        // Simulating ANSI color codes
        let line = "\u{1b}[32mmain\u{1b}[0m: server is \u{1b}[1mlistening\u{1b}[0m on \u{1b}[34mhttp\u{1b}[0m://0.0.0.0:8099".to_string();

        process_log_line(
            line,
            log_buffer,
            server_state.clone(),
            LogSource::Stdout,
            None,
            1,
            Some(8099),
        );

        let state = server_state.lock().unwrap();
        assert!(state.is_ready, "Server should be ready with ANSI codes");
    }

    #[test]
    fn test_process_log_line_not_ready() {
        let log_buffer: LogBuffer = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let server_state: ServerStateHandle = Arc::new(std::sync::Mutex::new(ServerState {
            is_ready: false,
            generation: 1,
        }));
        let line = "Some random log line".to_string();

        process_log_line(
            line,
            log_buffer,
            server_state.clone(),
            LogSource::Stdout,
            None,
            1,
            Some(8099),
        );

        let state = server_state.lock().unwrap();
        assert!(
            !state.is_ready,
            "Server should NOT be ready with random log"
        );
    }

    #[test]
    fn test_process_log_line_generation_mismatch() {
        let log_buffer: LogBuffer = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let server_state: ServerStateHandle = Arc::new(std::sync::Mutex::new(ServerState {
            is_ready: false,
            generation: 2, // Mismatch
        }));
        let line = "main: server is listening on http://0.0.0.0:8099".to_string();

        process_log_line(
            line,
            log_buffer,
            server_state.clone(),
            LogSource::Stdout,
            None,
            1, // Mismatch
            Some(8099),
        );

        let state = server_state.lock().unwrap();
        assert!(
            !state.is_ready,
            "Server should NOT be ready if generation mismatched"
        );
    }
}
