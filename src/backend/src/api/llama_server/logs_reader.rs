use crate::api::llama_server::types::{LogBuffer, LogEntry, LogSource, ServerStateHandle};
use crate::api::llama_server::websocket::WebSocketState;
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
    read_stream_generic(
        stream,
        log_buffer,
        server_state,
        LogSource::Stdout,
        ws_state,
        generation,
        port,
    );
}

fn read_stderr_stream(
    stream: ChildStderr,
    log_buffer: LogBuffer,
    server_state: ServerStateHandle,
    ws_state: Option<Arc<WebSocketState>>,
    generation: u32,
    port: Option<u16>,
) {
    read_stream_generic(
        stream,
        log_buffer,
        server_state,
        LogSource::Stderr,
        ws_state,
        generation,
        port,
    );
}

fn read_stream_generic<R: std::io::Read>(
    stream: R,
    log_buffer: LogBuffer,
    server_state: ServerStateHandle,
    source: LogSource,
    ws_state: Option<Arc<WebSocketState>>,
    generation: u32,
    port: Option<u16>,
) {
    let mut reader = std::io::BufReader::new(stream);
    let mut buf = [0; 1024];
    let mut line_buf = Vec::new();

    loop {
        use std::io::Read;
        match reader.read(&mut buf) {
            Ok(0) => {
                if !line_buf.is_empty() {
                    let line = String::from_utf8_lossy(&line_buf).into_owned();
                    process_log_line(
                        line,
                        log_buffer.clone(),
                        server_state.clone(),
                        source.clone(),
                        ws_state.clone(),
                        generation,
                        port,
                    );
                }
                break;
            }
            Ok(n) => {
                for &b in &buf[..n] {
                    if b == b'\n' || b == b'\r' {
                        if !line_buf.is_empty() {
                            let line = String::from_utf8_lossy(&line_buf).into_owned();
                            process_log_line(
                                line,
                                log_buffer.clone(),
                                server_state.clone(),
                                source.clone(),
                                ws_state.clone(),
                                generation,
                                port,
                            );
                            line_buf.clear();
                        }
                    } else {
                        line_buf.push(b);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading {:?} stream: {}", source, e);
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
        ((line.contains("main") || line.contains("llama_server") || line.contains("srv"))
            && line.contains("listening")
            && line.contains("http"))
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
    use actix_web::web;
    use std::collections::VecDeque;
    use std::io::Cursor;
    use std::sync::Mutex;
    use tokio::sync::mpsc::UnboundedReceiver;

    /// Everything the reader writes into, plus a WebSocket client so broadcasts
    /// can be asserted on.
    struct Harness {
        log_buffer: LogBuffer,
        server_state: ServerStateHandle,
        ws_state: Arc<WebSocketState>,
        logs_rx: UnboundedReceiver<String>,
        status_rx: UnboundedReceiver<String>,
    }

    fn harness(generation: u32) -> Harness {
        let log_buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        let server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState {
            is_ready: false,
            generation,
        }));
        let ws_state = Arc::new(WebSocketState::new(
            web::Data::new(log_buffer.clone()),
            web::Data::new(crate::api::llama_server::types::ProcessHandle(Arc::new(
                Mutex::new(None),
            ))),
            web::Data::new(server_state.clone()),
        ));

        let (logs_tx, logs_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (status_tx, status_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        ws_state.add_logs_client("logs-client".to_string(), logs_tx);
        ws_state.add_status_client("status-client".to_string(), status_tx);

        Harness {
            log_buffer,
            server_state,
            ws_state,
            logs_rx,
            status_rx,
        }
    }

    impl Harness {
        fn lines(&self) -> Vec<String> {
            self.log_buffer
                .lock()
                .unwrap()
                .iter()
                .map(|e| e.line.clone())
                .collect()
        }

        fn drain_logs(&mut self) -> Vec<serde_json::Value> {
            drain(&mut self.logs_rx)
        }

        fn drain_status(&mut self) -> Vec<serde_json::Value> {
            drain(&mut self.status_rx)
        }
    }

    fn drain(rx: &mut UnboundedReceiver<String>) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            out.push(serde_json::from_str(&msg).unwrap());
        }
        out
    }

    /// Feeds canned bytes through the real stream reader, no child process needed.
    fn feed(h: &Harness, input: &str, source: LogSource, generation: u32) {
        read_stream_generic(
            Cursor::new(input.as_bytes().to_vec()),
            h.log_buffer.clone(),
            h.server_state.clone(),
            source,
            Some(h.ws_state.clone()),
            generation,
            Some(8099),
        );
    }

    /// A reader that always fails, to drive the reader's error arm.
    struct FailingReader;

    impl std::io::Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "pipe closed",
            ))
        }
    }

    #[test]
    fn test_read_stream_splits_on_newlines_and_flushes_a_trailing_partial_line() {
        let h = harness(1);

        feed(
            &h,
            "first\nsecond\nno trailing newline",
            LogSource::Stdout,
            1,
        );

        assert_eq!(
            h.lines(),
            vec!["first", "second", "no trailing newline"],
            "the final unterminated line must still be flushed"
        );
    }

    #[test]
    fn test_read_stream_treats_carriage_returns_as_line_breaks_and_drops_blank_lines() {
        let h = harness(1);

        feed(&h, "a\r\nb\r\n\r\n\nc\n", LogSource::Stderr, 1);

        assert_eq!(h.lines(), vec!["a", "b", "c"]);
    }

    #[test]
    fn test_read_stream_handles_lines_longer_than_the_read_buffer() {
        let h = harness(1);
        let long_line = "x".repeat(4096);

        feed(&h, &format!("{}\n", long_line), LogSource::Stdout, 1);

        assert_eq!(h.lines(), vec![long_line]);
    }

    #[test]
    fn test_read_stream_of_empty_input_records_nothing() {
        let h = harness(1);

        feed(&h, "", LogSource::Stdout, 1);

        assert!(h.lines().is_empty());
    }

    #[test]
    fn test_read_stream_stops_when_the_underlying_reader_errors() {
        let h = harness(1);

        read_stream_generic(
            FailingReader,
            h.log_buffer.clone(),
            h.server_state.clone(),
            LogSource::Stderr,
            Some(h.ws_state.clone()),
            1,
            Some(8099),
        );

        assert!(
            h.lines().is_empty(),
            "a failed read must not invent log lines"
        );
    }

    #[test]
    fn test_read_stream_keeps_only_the_most_recent_thousand_lines() {
        let h = harness(1);
        let input: String = (0..1005).map(|i| format!("line-{}\n", i)).collect();

        feed(&h, &input, LogSource::Stdout, 1);

        let lines = h.lines();
        assert_eq!(lines.len(), 1000);
        assert_eq!(lines.first().unwrap(), "line-5");
        assert_eq!(lines.last().unwrap(), "line-1004");
    }

    #[test]
    fn test_read_stream_ignores_everything_once_the_generation_moved_on() {
        let h = harness(9);

        feed(&h, "orphaned line\n", LogSource::Stdout, 8);

        assert!(h.lines().is_empty());
    }

    #[test]
    fn test_process_log_line_broadcasts_the_line_and_the_ready_status() {
        let mut h = harness(1);

        feed(
            &h,
            "srv  llama_server: listening on http://0.0.0.0:8099\n",
            LogSource::Stdout,
            1,
        );

        assert!(h.server_state.lock().unwrap().is_ready);

        let logs = h.drain_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0]["type"], "log");
        assert_eq!(logs[0]["log"]["source"], "stdout");
        assert!(logs[0]["log"]["line"].as_str().unwrap().contains("srv"));

        let status = h.drain_status();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0]["type"], "status");
        assert_eq!(status[0]["active"], true);
        assert_eq!(status[0]["port"], 8099);
    }

    #[test]
    fn test_process_log_line_falls_back_to_port_8080_when_no_port_is_configured() {
        let mut h = harness(1);

        process_log_line(
            "HTTP server listening".to_string(),
            h.log_buffer.clone(),
            h.server_state.clone(),
            LogSource::Stdout,
            Some(h.ws_state.clone()),
            1,
            None,
        );

        let status = h.drain_status();
        assert_eq!(status.len(), 1);
        assert_eq!(status[0]["port"], 8080);
    }

    #[test]
    fn test_process_log_line_labels_stderr_broadcasts_correctly() {
        let mut h = harness(1);

        feed(&h, "loading weights\n", LogSource::Stderr, 1);

        let logs = h.drain_logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0]["log"]["source"], "stderr");
        assert!(
            h.drain_status().is_empty(),
            "an ordinary line must not announce readiness"
        );
    }

    #[test]
    fn test_process_log_line_does_not_treat_a_partial_listening_match_as_ready() {
        let mut h = harness(1);

        // Mentions "listening" but has none of the required companion tokens.
        feed(&h, "waiting: not listening yet\n", LogSource::Stdout, 1);

        assert!(!h.server_state.lock().unwrap().is_ready);
        assert_eq!(h.lines(), vec!["waiting: not listening yet"]);
        assert_eq!(h.drain_logs().len(), 1);
        assert!(h.drain_status().is_empty());
    }

    #[test]
    fn test_process_log_line_records_the_line_even_without_a_websocket() {
        let h = harness(1);

        process_log_line(
            "no websocket attached".to_string(),
            h.log_buffer.clone(),
            h.server_state.clone(),
            LogSource::Stdout,
            None,
            1,
            Some(8099),
        );

        assert_eq!(h.lines(), vec!["no websocket attached"]);
    }

    #[test]
    fn test_spawn_log_reader_consumes_both_real_pipes_of_a_harmless_child() {
        let h = harness(1);
        // `sh` is used purely to produce piped output. This is never
        // `llama-server`: the reader only cares about the bytes on the pipes.
        let mut child = std::process::Command::new("sh")
            .arg("-c")
            .arg("printf 'main: server is listening on http://0.0.0.0:8099\\n'; printf 'a warning\\n' >&2")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("`sh` must be available to run these tests");

        spawn_log_reader(
            child.stdout.take(),
            child.stderr.take(),
            h.log_buffer.clone(),
            h.server_state.clone(),
            Some(h.ws_state.clone()),
            Some(8099),
        );
        let _ = child.wait();

        // The reader threads are detached, so poll for the expected lines.
        let mut lines = Vec::new();
        for _ in 0..200 {
            lines = h.lines();
            if lines.len() >= 2 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(
            lines.iter().any(|l| l.contains("server is listening")),
            "stdout should have been read: {:?}",
            lines
        );
        assert!(
            lines.iter().any(|l| l == "a warning"),
            "stderr should have been read: {:?}",
            lines
        );
        assert!(h.server_state.lock().unwrap().is_ready);
    }

    #[test]
    fn test_spawn_log_reader_with_no_pipes_does_nothing() {
        let h = harness(1);

        spawn_log_reader(
            None,
            None,
            h.log_buffer.clone(),
            h.server_state.clone(),
            Some(h.ws_state.clone()),
            Some(8099),
        );

        assert!(h.lines().is_empty());
        assert!(!h.server_state.lock().unwrap().is_ready);
    }

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
    fn test_process_log_line_readiness_new_format() {
        let log_buffer: LogBuffer = Arc::new(std::sync::Mutex::new(VecDeque::new()));
        let server_state: ServerStateHandle = Arc::new(std::sync::Mutex::new(ServerState {
            is_ready: false,
            generation: 1,
        }));
        let line = "0.01.411.337 I srv  llama_server: listening on http://0.0.0.0:8099".to_string();

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
            state.is_ready,
            "Server should be ready with new log line format"
        );
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
