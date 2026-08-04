use std::io::{BufRead, BufReader};
use std::process::{ChildStderr, ChildStdout};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::sd_server::types::{LogBuffer, LogEntry, LogSource, SDStateHandle};
use crate::api::sd_server::websocket::WebSocketState;

use crate::api::sd_server::storage::SDImagesStorage;

pub fn spawn_log_reader(
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    log_buffer: LogBuffer,
    sd_state: SDStateHandle,
    ws_state: Arc<WebSocketState>,
    storage: Arc<SDImagesStorage>,
) {
    if let Some(stdout_handle) = stdout {
        let log_buffer_clone = log_buffer.clone();
        let ws_state_clone = ws_state.clone();
        let sd_state_clone = sd_state.clone();
        let storage_clone = storage.clone();
        std::thread::spawn(move || {
            read_stream(
                stdout_handle,
                log_buffer_clone,
                ws_state_clone,
                sd_state_clone,
                LogSource::Stdout,
                storage_clone,
            );
        });
    }

    if let Some(stderr_handle) = stderr {
        let log_buffer_clone = log_buffer.clone();
        let ws_state_clone = ws_state.clone();
        let sd_state_clone = sd_state.clone();
        let storage_clone = storage.clone();
        std::thread::spawn(move || {
            read_stream(
                stderr_handle,
                log_buffer_clone,
                ws_state_clone,
                sd_state_clone,
                LogSource::Stderr,
                storage_clone,
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
    storage: Arc<SDImagesStorage>,
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
            state.pending_filename = None; // clear pending

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
        } else if line.contains("out of memory")
            || line.contains("cudaMalloc failed")
            || line.contains("failed to allocate")
        {
            // OOM or Allocation Error detected
            println!("❌ Detected OOM/Allocation error in SD log: {}", line);
            let mut state = sd_state.lock().unwrap();
            state.is_generating = false;
            let pending = state.pending_filename.clone();
            state.pending_filename = None;
            drop(state);

            // Cleanup DB
            if let Some(filename) = pending {
                let storage_clone = storage.clone();
                // Spawn a standard thread to run a small runtime for cleanup
                // This avoids "spawn_local called from outside of a task::LocalSet" panic
                std::thread::spawn(move || {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        rt.block_on(async {
                            println!("🧹 Cleaning up pending image from DB: {}", filename);
                            if let Err(e) = storage_clone.delete_image(&filename).await {
                                println!("⚠️ Failed to delete pending image: {:?}", e);
                            }
                        });
                    }
                });
            }

            ws_state.broadcast_error("Out of Memory / Allocation Failed".to_string());
            ws_state.broadcast_status(false, None);
        }
    }

    // Stream ended (Process likely exited)
    // Ensure we reset generating state if it crashed without success/error handling
    {
        let mut state = sd_state.lock().unwrap();
        if state.is_generating {
            println!("⚠️ SD Log stream ended unexpectedly while generating. Marking as stopped.");
            state.is_generating = false;
            let pending = state.pending_filename.clone();
            state.pending_filename = None;
            drop(state);

            // Cleanup DB
            if let Some(filename) = pending {
                let storage_clone = storage.clone();
                // Spawn a standard thread to run a small runtime for cleanup
                std::thread::spawn(move || {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        rt.block_on(async {
                            println!("🧹 Cleaning up pending image from DB (Crash): {}", filename);
                            if let Err(e) = storage_clone.delete_image(&filename).await {
                                println!("⚠️ Failed to delete pending image: {:?}", e);
                            }
                        });
                    }
                });
            }

            ws_state.broadcast_error("Process crashed or exited unexpectedly".to_string());
            ws_state.broadcast_status(false, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::sd_server::storage::{new_file_test_storage, test_image};
    use crate::api::sd_server::types::{SDConfig, SDState};
    use actix_web::web;
    use std::collections::VecDeque;
    use std::io::Cursor;
    use std::sync::Mutex;
    use tokio::sync::mpsc::UnboundedReceiver;

    /// Everything `read_stream` needs, plus a receiver wired up as a WebSocket
    /// client so broadcasts can be asserted on.
    struct Harness {
        log_buffer: LogBuffer,
        sd_state: SDStateHandle,
        ws_state: Arc<WebSocketState>,
        storage: Arc<SDImagesStorage>,
        rx: UnboundedReceiver<String>,
        // The cleanup paths reach the DB from a freshly built runtime on a
        // detached thread, which an in-memory database cannot survive.
        _db_dir: tempfile::TempDir,
    }

    async fn harness(initial_state: SDState) -> Harness {
        let log_buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));
        let sd_state: SDStateHandle = Arc::new(Mutex::new(initial_state));
        let ws_state = Arc::new(WebSocketState::new(
            web::Data::new(log_buffer.clone()),
            web::Data::new(Arc::new(Mutex::new(None))),
            web::Data::new(Arc::new(Mutex::new(SDConfig::default()))),
            web::Data::new(sd_state.clone()),
        ));

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        ws_state.add_client("test-client".to_string(), tx);

        let (db_dir, storage) = new_file_test_storage().await;

        Harness {
            log_buffer,
            sd_state,
            ws_state,
            storage: Arc::new(storage),
            rx,
            _db_dir: db_dir,
        }
    }

    fn idle() -> SDState {
        SDState {
            is_generating: false,
            current_output_file: None,
            pending_filename: None,
        }
    }

    fn generating(pending: &str) -> SDState {
        SDState {
            is_generating: true,
            current_output_file: None,
            pending_filename: Some(pending.to_string()),
        }
    }

    fn feed(h: &Harness, input: &str, source: LogSource) {
        read_stream(
            Cursor::new(input.as_bytes().to_vec()),
            h.log_buffer.clone(),
            h.ws_state.clone(),
            h.sd_state.clone(),
            source,
            h.storage.clone(),
        );
    }

    fn drain(rx: &mut UnboundedReceiver<String>) -> Vec<String> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            out.push(msg);
        }
        out
    }

    /// The DB cleanup for a pending image happens on a detached thread, so give
    /// it a bounded window to land before asserting.
    async fn wait_until_empty(storage: &SDImagesStorage) -> bool {
        for _ in 0..100 {
            if storage.get_images().await.unwrap().is_empty() {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        false
    }

    #[tokio::test]
    async fn test_lines_are_buffered_and_broadcast() {
        let mut h = harness(idle()).await;

        feed(&h, "loading model\nsampling\n", LogSource::Stdout);

        let buffer = h.log_buffer.lock().unwrap();
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer[0].line, "loading model");
        assert_eq!(buffer[0].source, LogSource::Stdout);
        assert_eq!(buffer[1].line, "sampling");
        assert!(buffer[0].timestamp > 0);
        drop(buffer);

        let messages = drain(&mut h.rx);
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains(r#""type":"log""#));
        assert!(messages[0].contains("loading model"));
        assert!(messages[0].contains(r#""source":"stdout""#));
    }

    #[tokio::test]
    async fn test_stderr_lines_are_tagged_as_stderr() {
        let mut h = harness(idle()).await;

        feed(&h, "warning: slow\n", LogSource::Stderr);

        assert_eq!(h.log_buffer.lock().unwrap()[0].source, LogSource::Stderr);
        let messages = drain(&mut h.rx);
        assert!(messages[0].contains(r#""source":"stderr""#));
    }

    #[tokio::test]
    async fn test_buffer_is_capped_at_1000_entries_dropping_the_oldest() {
        let h = harness(idle()).await;

        let input: String = (0..1100).map(|i| format!("line {}\n", i)).collect();
        feed(&h, &input, LogSource::Stdout);

        let buffer = h.log_buffer.lock().unwrap();
        assert_eq!(buffer.len(), 1000);
        assert_eq!(buffer.front().unwrap().line, "line 100");
        assert_eq!(buffer.back().unwrap().line, "line 1099");
    }

    #[tokio::test]
    async fn test_success_line_clears_generating_and_records_the_public_path() {
        let mut h = harness(generating("out.png")).await;

        feed(
            &h,
            "[INFO ] main.cpp:421  - save result image 0 to './images/out.png' (success)\n",
            LogSource::Stdout,
        );

        let state = h.sd_state.lock().unwrap();
        assert!(!state.is_generating);
        assert!(state.pending_filename.is_none());
        assert_eq!(
            state.current_output_file.as_deref(),
            Some("/public/out.png")
        );
        drop(state);

        let messages = drain(&mut h.rx);
        // One log line plus one status broadcast.
        assert_eq!(messages.len(), 2);
        assert!(messages[1].contains(r#""type":"status""#));
        assert!(messages[1].contains(r#""is_generating":false"#));
        assert!(messages[1].contains("/public/out.png"));
    }

    #[tokio::test]
    async fn test_success_line_without_a_quoted_path_leaves_the_file_unset() {
        let mut h = harness(generating("out.png")).await;

        feed(&h, "save result image 0 success\n", LogSource::Stdout);

        let state = h.sd_state.lock().unwrap();
        assert!(!state.is_generating);
        assert!(state.current_output_file.is_none());
        drop(state);

        let messages = drain(&mut h.rx);
        assert!(messages[1].contains(r#""current_file":null"#));
    }

    #[tokio::test]
    async fn test_success_line_with_an_empty_path_falls_back_to_unknown_png() {
        let h = harness(generating("out.png")).await;

        feed(
            &h,
            "save result image 0 to '' (success)\n",
            LogSource::Stdout,
        );

        assert_eq!(
            h.sd_state.lock().unwrap().current_output_file.as_deref(),
            Some("/public/unknown.png")
        );
    }

    #[tokio::test]
    async fn test_oom_line_reports_an_error_and_deletes_the_pending_row() {
        let mut h = harness(generating("pending.png")).await;
        h.storage
            .add_image(test_image("pending.png", 1))
            .await
            .unwrap();

        feed(
            &h,
            "ggml_cuda_host_malloc: failed to allocate 1024 MB\n",
            LogSource::Stdout,
        );

        let (is_generating, pending) = {
            let state = h.sd_state.lock().unwrap();
            (state.is_generating, state.pending_filename.clone())
        };
        assert!(!is_generating);
        assert!(pending.is_none());

        let messages = drain(&mut h.rx);
        assert!(messages
            .iter()
            .any(|m| m.contains(r#""type":"error""#)
                && m.contains("Out of Memory / Allocation Failed")));
        assert!(messages.iter().any(|m| m.contains(r#""type":"status""#)));

        assert!(
            wait_until_empty(&h.storage).await,
            "the pending image row should have been cleaned up"
        );
    }

    #[tokio::test]
    async fn test_oom_line_without_a_pending_filename_still_reports_the_error() {
        let mut h = harness(SDState {
            is_generating: true,
            current_output_file: None,
            pending_filename: None,
        })
        .await;

        feed(&h, "CUDA error: out of memory\n", LogSource::Stdout);

        assert!(!h.sd_state.lock().unwrap().is_generating);
        let messages = drain(&mut h.rx);
        assert!(messages.iter().any(|m| m.contains(r#""type":"error""#)));
    }

    #[tokio::test]
    async fn test_stream_ending_mid_generation_is_treated_as_a_crash() {
        let mut h = harness(generating("half-done.png")).await;
        h.storage
            .add_image(test_image("half-done.png", 1))
            .await
            .unwrap();

        // No success and no OOM marker - the process just went away.
        feed(&h, "sampling step 3/20\n", LogSource::Stdout);

        let (is_generating, pending) = {
            let state = h.sd_state.lock().unwrap();
            (state.is_generating, state.pending_filename.clone())
        };
        assert!(!is_generating);
        assert!(pending.is_none());

        let messages = drain(&mut h.rx);
        assert!(messages
            .iter()
            .any(|m| m.contains("Process crashed or exited unexpectedly")));

        assert!(
            wait_until_empty(&h.storage).await,
            "the pending image row should have been cleaned up"
        );
    }

    #[tokio::test]
    async fn test_stream_ending_while_idle_broadcasts_nothing_extra() {
        let mut h = harness(idle()).await;

        feed(&h, "just a log line\n", LogSource::Stdout);

        // Only the log line itself - no status/error broadcast.
        let messages = drain(&mut h.rx);
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains(r#""type":"log""#));
        assert!(!h.sd_state.lock().unwrap().is_generating);
    }

    #[tokio::test]
    async fn test_empty_stream_while_idle_is_a_no_op() {
        let mut h = harness(idle()).await;

        feed(&h, "", LogSource::Stdout);

        assert!(h.log_buffer.lock().unwrap().is_empty());
        assert!(drain(&mut h.rx).is_empty());
    }

    #[tokio::test]
    async fn test_spawn_log_reader_without_streams_does_nothing() {
        let h = harness(idle()).await;

        // Both handles are None (the child had no piped stdio), so no reader
        // threads are started and nothing is buffered.
        spawn_log_reader(
            None,
            None,
            h.log_buffer.clone(),
            h.sd_state.clone(),
            h.ws_state.clone(),
            h.storage.clone(),
        );

        assert!(h.log_buffer.lock().unwrap().is_empty());
    }
}
