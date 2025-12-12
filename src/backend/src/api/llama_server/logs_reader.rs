use crate::api::llama_server::types::{LogBuffer, LogEntry, LogSource, ServerStateHandle};
use std::io::{BufRead, BufReader};
use std::process::{ChildStderr, ChildStdout};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn spawn_log_reader(
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    log_buffer: LogBuffer,
    server_state: ServerStateHandle,
) {
    if let Some(stdout_handle) = stdout {
        let log_buffer_clone = log_buffer.clone();
        let server_state_clone = server_state.clone();
        std::thread::spawn(move || {
            read_stdout_stream(stdout_handle, log_buffer_clone, server_state_clone);
        });
    }

    if let Some(stderr_handle) = stderr {
        let log_buffer_clone = log_buffer.clone();
        let server_state_clone = server_state.clone();
        std::thread::spawn(move || {
            read_stderr_stream(stderr_handle, log_buffer_clone, server_state_clone);
        });
    }
}

fn read_stdout_stream(stream: ChildStdout, log_buffer: LogBuffer, server_state: ServerStateHandle) {
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
                );
            }
            Err(e) => {
                eprintln!("❌ Error reading stdout line: {}", e);
                break;
            }
        }
    }
}

fn read_stderr_stream(stream: ChildStderr, log_buffer: LogBuffer, server_state: ServerStateHandle) {
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
                );
            }
            Err(e) => {
                eprintln!("❌ Error reading stderr line: {}", e);
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
) {
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
        buffer.push_back(log_entry);
        if buffer.len() > 1000 {
            buffer.pop_front();
        }
    }

    // Check if server is ready
    if line.contains("main: server is listening on http://127.0.0.1:8080") {
        println!("✅ Detected server ready message!");
        let mut state = server_state.lock().unwrap();
        state.is_ready = true;
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
