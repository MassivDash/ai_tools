use std::io::{BufRead, BufReader};
use std::process::{ChildStderr, ChildStdout};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::sd_server::types::{LogBuffer, LogEntry, LogSource, SDStateHandle};
use crate::api::sd_server::websocket::WebSocketState;

pub fn spawn_log_reader(
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    log_buffer: LogBuffer,
    sd_state: SDStateHandle,
    ws_state: Arc<WebSocketState>,
) {
    if let Some(stdout_handle) = stdout {
        let log_buffer_clone = log_buffer.clone();
        let ws_state_clone = ws_state.clone();
        let sd_state_clone = sd_state.clone();
        std::thread::spawn(move || {
            read_stream(
                stdout_handle,
                log_buffer_clone,
                ws_state_clone,
                sd_state_clone,
                LogSource::Stdout,
            );
        });
    }

    if let Some(stderr_handle) = stderr {
        let log_buffer_clone = log_buffer.clone();
        let ws_state_clone = ws_state.clone();
        let sd_state_clone = sd_state.clone();
        std::thread::spawn(move || {
            read_stream(
                stderr_handle,
                log_buffer_clone,
                ws_state_clone,
                sd_state_clone,
                LogSource::Stderr,
            );
        });
    }
}

fn read_stream<R: std::io::Read>(
    stream: R,
    log_buffer: LogBuffer,
    ws_state: Arc<WebSocketState>,
    sd_state: SDStateHandle,
    source: LogSource,
) {
    let reader = BufReader::new(stream);

    for line in reader.lines().map_while(Result::ok) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // rest of the loop body is the same, but indentation reduced?
        // Actually, since I'm targeting the whole loop block, I should just remove the `if let Ok(line)` wrap.
        // But `replace_file_content` with a huge block is risky.
        // Wait, the tool output suggested:
        /*
         56 ~     for line in reader.lines().flatten() {
         57 +         let timestamp = SystemTime::now()
         ...
        */
        let log_entry = LogEntry {
            timestamp,
            line: line.clone(),
            source: source.clone(),
        };

        // Buffer
        {
            let mut buffer = log_buffer.lock().unwrap();
            buffer.push_back(log_entry.clone());
            if buffer.len() > 1000 {
                buffer.pop_front();
            }
        }

        // Broadcast
        use crate::api::sd_server::websocket::LogLine;
        let log_line = LogLine {
            timestamp,
            line: line.clone(),
            source: match source {
                LogSource::Stdout => "stdout".to_string(),
                LogSource::Stderr => "stderr".to_string(),
            },
        };
        ws_state.broadcast_log(log_line);

        // Attempt to detect completion or progress based on log lines
        // Example: [INFO ] main.cpp:421  - save result image 0 to './images/randomName.png' (success)
        if line.contains("save result image")
            && (line.contains("(success)") || line.contains("success"))
        {
            // Generation finished success
            let mut state = sd_state.lock().unwrap();
            state.is_generating = false;

            // Extract filename if possible
            // line format: ... save result image 0 to './images/randomName.png' (success)
            if let Some(start) = line.find("to '") {
                if let Some(end) = line[start + 4..].find("'") {
                    let full_path = &line[start + 4..start + 4 + end];
                    // Extract just the filename to serve via /public
                    let filename = std::path::Path::new(full_path)
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown.png".to_string());

                    state.current_output_file = Some(format!("/public/{}", filename));
                }
            }

            let file = state.current_output_file.clone();
            drop(state); // release lock before broadcast

            ws_state.broadcast_status(false, file);
        }
    }
}
