use crate::api::agent::core::types::{ToolCall, ToolCallResult, ToolType};
use crate::api::agent::tools::framework::agent_tool::{AgentTool, ToolCategory, ToolMetadata};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

pub struct SystemCommandTool {
    metadata: ToolMetadata,
}

impl SystemCommandTool {
    pub fn new() -> Self {
        Self {
            metadata: ToolMetadata {
                id: "system_command".to_string(),
                name: "System Tools".to_string(),
                tool_type: ToolType::SystemCommand,
                description: "Execute safe, everyday Linux system commands (e.g., search files, view processes, check system status).".to_string(),
                category: ToolCategory::Utility,
            },
        }
    }
}

impl Default for SystemCommandTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentTool for SystemCommandTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    fn get_function_definition(&self) -> serde_json::Value {
        json!({
            "name": "system_command",
            "description": "Execute safe, everyday Linux system commands (e.g., search files, view processes, check system status).",
            "parameters": {
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The specific command to run. Valid options: 'search_file', 'open_folder', 'open_vscode', 'open_vim', 'list_top_processes', 'grep_search', 'system_status', 'network_ports', 'ping_host'",
                        "enum": ["search_file", "open_folder", "open_vscode", "open_vim", "list_top_processes", "grep_search", "system_status", "network_ports", "ping_host"]
                    },
                    "path": {
                        "type": "string",
                        "description": "The directory or file path for 'search_file', 'open_folder', 'grep_search', 'open_vscode', or 'open_vim'. Supports '~' for the home directory, e.g. '~/Documents' or '/home/user/Documents'. Defaults to the home directory if omitted."
                    },
                    "query": {
                        "type": "string",
                        "description": "The search pattern for 'search_file' (file name, e.g. '*.jks', matched case-insensitively) or 'grep_search' (text content). For 'ping_host', the target hostname or IP address."
                    }
                },
                "required": ["command"]
            }
        })
    }

    async fn execute(&self, tool_call: &ToolCall) -> Result<ToolCallResult> {
        let args: serde_json::Value =
            serde_json::from_str(&tool_call.function.arguments).unwrap_or(json!({}));

        let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let raw_path = args.get("path").and_then(|v| v.as_str()).unwrap_or("~");
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

        let path = expand_path(raw_path);

        let timeout_duration = Duration::from_secs(30);

        let output = match command {
            "search_file" => {
                if query.is_empty() {
                    return Ok(ToolCallResult {
                        tool_name: self.metadata.name.clone(),
                        result: "Error: 'query' parameter is required for search_file.".to_string(),
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
                if let Some(err) = check_path_exists(&path) {
                    err
                } else {
                    let pattern = as_glob_pattern(query);
                    let mut cmd = Command::new("find");
                    // -iname: match case-insensitively so "*.jks" also finds "*.JKS"
                    cmd.arg(&path).arg("-iname").arg(&pattern);
                    let raw = timeout(timeout_duration, cmd.output()).await;
                    process_matches_output(
                        raw,
                        &format!("No files matching '{}' found under '{}'.", pattern, path),
                    )
                }
            }
            "open_folder" => {
                let mut cmd = Command::new("xdg-open");
                cmd.arg(&path);
                match timeout(timeout_duration, cmd.output()).await {
                    Ok(Ok(o)) if o.status.success() => "Opened successfully.".to_string(),
                    Ok(Ok(o)) => format!("Failed to open: {}", String::from_utf8_lossy(&o.stderr)),
                    Ok(Err(e)) => format!("Failed to execute xdg-open: {}", e),
                    Err(_) => "Command timed out.".to_string(),
                }
            }
            "open_vscode" => {
                if let Some(err) = check_path_or_parent_exists(&path) {
                    err
                } else {
                    let mut cmd = Command::new("code");
                    cmd.arg(&path);
                    match timeout(timeout_duration, cmd.output()).await {
                        Ok(Ok(o)) if o.status.success() => {
                            format!("Opened '{}' in VS Code.", path)
                        }
                        Ok(Ok(o)) => {
                            format!("Failed to open in VS Code: {}", String::from_utf8_lossy(&o.stderr))
                        }
                        Ok(Err(e)) => format!(
                            "Failed to execute 'code' (is VS Code's CLI installed and on PATH?): {}",
                            e
                        ),
                        Err(_) => "Command timed out.".to_string(),
                    }
                }
            }
            "open_vim" => {
                if let Some(err) = check_path_or_parent_exists(&path) {
                    err
                } else {
                    open_in_terminal_editor("vim", &path, timeout_duration).await
                }
            }
            "list_top_processes" => {
                let mut cmd = Command::new("ps");
                cmd.args(["-eo", "pid,ppid,cmd,%mem,%cpu", "--sort=-%cpu"]);

                let res = process_output(timeout(timeout_duration, cmd.output()).await);
                // Get the first 15 lines in Rust instead of shelling out to `head`
                let lines: Vec<&str> = res.lines().take(16).collect();
                lines.join("\n")
            }
            "grep_search" => {
                if query.is_empty() {
                    return Ok(ToolCallResult {
                        tool_name: self.metadata.name.clone(),
                        result: "Error: 'query' parameter is required for grep_search.".to_string(),
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
                if let Some(err) = check_path_exists(&path) {
                    err
                } else {
                    let mut cmd = Command::new("grep");
                    cmd.arg("-rn").arg(query).arg(&path);
                    let raw = timeout(timeout_duration, cmd.output()).await;
                    process_matches_output(
                        raw,
                        &format!("No matches found for '{}' under '{}'.", query, path),
                    )
                }
            }
            "system_status" => {
                let mut df_cmd = Command::new("df");
                df_cmd.arg("-h");
                let df_out = timeout(timeout_duration, df_cmd.output()).await;

                let mut free_cmd = Command::new("free");
                free_cmd.arg("-m");
                let free_out = timeout(timeout_duration, free_cmd.output()).await;

                let mut result = String::new();
                result.push_str("--- Disk Usage ---\n");
                result.push_str(&process_output(df_out));
                result.push_str("\n--- Memory Usage ---\n");
                result.push_str(&process_output(free_out));
                result
            }
            "network_ports" => {
                let mut cmd = Command::new("ss");
                cmd.arg("-tuln");
                process_output(timeout(timeout_duration, cmd.output()).await)
            }
            "ping_host" => {
                if query.is_empty() {
                    return Ok(ToolCallResult {
                        tool_name: self.metadata.name.clone(),
                        result: "Error: 'query' parameter (the hostname or IP to ping) is required for ping_host.".to_string(),
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
                let mut cmd = Command::new("ping");
                if cfg!(windows) {
                    // -n 4: four probes; -w 2000: 2000ms per-reply wait (Windows'
                    // ping takes milliseconds and uses different flag letters
                    // than the Unix/BSD/GNU ping below).
                    cmd.args(["-n", "4", "-w", "2000"]).arg(query);
                } else {
                    // -c 4: four probes and stop (don't run forever); -W 2: 2s per-reply wait.
                    cmd.args(["-c", "4", "-W", "2"]).arg(query);
                }
                let raw = timeout(timeout_duration, cmd.output()).await;
                // Like find/grep, ping exits non-zero on 100% packet loss even
                // though it already printed a useful summary to stdout - trust
                // stdout when present rather than reporting that as a hard error.
                process_matches_output(raw, &format!("Could not resolve or reach '{}'.", query))
            }
            _ => format!("Unknown command: {}", command),
        };

        Ok(ToolCallResult {
            tool_name: self.metadata.name.clone(),
            result: output,
            tool_call_id: Some(tool_call.id.clone()),
        })
    }
}

/// Expands a leading `~` or `~/...` to the user's home directory.
///
/// `tokio::process::Command` never invokes a shell, so shells' own `~`
/// expansion never happens for arguments built here — without this, a path
/// like `~/Git/AndroStore` is passed to `find`/`grep` literally, and they
/// look for a directory actually named `~`.
fn expand_path(path: &str) -> String {
    if path == "~" || path.starts_with("~/") {
        // HOME is the Unix/macOS convention; Windows sets USERPROFILE instead
        // (and typically doesn't set HOME at all).
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            return path.replacen('~', &home, 1);
        }
    }
    path.to_string()
}

/// Turns a bare filename/extension into a `find -iname` glob pattern.
///
/// The LLM doesn't reliably include `*` wildcards (e.g. it may pass "jks" or
/// "spaceghost" for "find files ending in jks"), and `find -iname` treats its
/// argument as a literal name unless it already contains glob characters. So
/// a query with no glob metacharacter is wrapped in `*...*` to behave like an
/// intuitive substring/extension search instead of silently matching nothing.
fn as_glob_pattern(query: &str) -> String {
    if query.contains(['*', '?', '[']) {
        query.to_string()
    } else if let Some(ext) = query.strip_prefix('.') {
        format!("*.{}", ext)
    } else {
        format!("*{}*", query)
    }
}

/// Launches `editor path` inside a GUI terminal emulator, since terminal
/// editors like vim have no window of their own to open. Tries known
/// terminal emulators in order until one launches successfully.
///
/// Deliberately does *not* try `x-terminal-emulator`: on Debian/Ubuntu it
/// commonly resolves to a Perl wrapper around gnome-terminal that always adds
/// `--wait` (to mimic legacy xterm semantics), which would block this async
/// call until the user closes the terminal window instead of returning as
/// soon as the editor is launched.
async fn open_in_terminal_editor(editor: &str, path: &str, timeout_duration: Duration) -> String {
    const CANDIDATES: &[(&str, &str)] = &[
        ("gnome-terminal", "--"),
        ("konsole", "-e"),
        ("xfce4-terminal", "-x"),
        ("alacritty", "-e"),
        ("xterm", "-e"),
    ];

    for (terminal, flag) in CANDIDATES {
        let mut cmd = Command::new(terminal);
        cmd.arg(flag).arg(editor).arg(path);
        match timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(o)) if o.status.success() => {
                return format!("Opened '{}' in {} inside {}.", path, editor, terminal);
            }
            // Wrong flag dialect, terminal not installed, etc. - try the next one.
            Ok(Ok(_)) | Ok(Err(_)) => continue,
            Err(_) => continue,
        }
    }

    let tried: Vec<&str> = CANDIDATES.iter().map(|(t, _)| *t).collect();
    format!(
        "Could not find a terminal emulator to open {} in (tried: {}).",
        editor,
        tried.join(", ")
    )
}

/// Returns an error message if `path` doesn't exist, so failures are
/// reported clearly instead of as a raw `find`/`grep` stderr dump that can
/// read like "the file doesn't exist" when really the directory doesn't.
fn check_path_exists(path: &str) -> Option<String> {
    if std::path::Path::new(path).exists() {
        None
    } else {
        Some(format!("Error: path '{}' does not exist.", path))
    }
}

/// Like `check_path_exists`, but for opening a path in an editor: the path
/// itself is allowed not to exist yet (creating a new file is normal editor
/// usage) as long as its parent directory does, so we only reject paths that
/// couldn't possibly be created (e.g. a typo'd directory).
fn check_path_or_parent_exists(path: &str) -> Option<String> {
    let p = std::path::Path::new(path);
    if p.exists() {
        return None;
    }
    match p.parent() {
        Some(parent) if parent.exists() => None,
        _ => Some(format!(
            "Error: '{}' does not exist and its parent directory doesn't either.",
            path
        )),
    }
}

/// Formats output for `find`/`grep`-style commands, where a non-zero exit
/// code does not mean the command failed: `find` exits 1 whenever it hits a
/// permission-denied subdirectory (extremely common when searching broad
/// paths like `/` or `~`) even though it already printed real matches to
/// stdout, and `grep` exits 1 simply to mean "no lines matched". Unlike
/// `process_output`, this always trusts stdout when it's non-empty instead
/// of discarding it for a non-zero exit status.
fn process_matches_output(
    timeout_result: Result<std::io::Result<std::process::Output>, tokio::time::error::Elapsed>,
    no_matches_message: &str,
) -> String {
    match timeout_result {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if !stdout.trim().is_empty() {
                stdout.into_owned()
            } else {
                no_matches_message.to_string()
            }
        }
        Ok(Err(e)) => format!("Failed to execute command: {}", e),
        Err(_) => "Command timed out after 30 seconds.".to_string(),
    }
}

fn process_output(
    timeout_result: Result<std::io::Result<std::process::Output>, tokio::time::error::Elapsed>,
) -> String {
    match timeout_result {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                if stdout.trim().is_empty() {
                    "Command executed successfully (no output).".to_string()
                } else {
                    stdout.into_owned()
                }
            } else {
                format!("Error exit {}:\n{}", out.status.code().unwrap_or(1), stderr)
            }
        }
        Ok(Err(e)) => format!("Failed to execute command: {}", e),
        Err(_) => "Command timed out after 30 seconds.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::agent::core::types::FunctionCall;

    fn call(args: serde_json::Value) -> ToolCall {
        ToolCall {
            id: "test-call".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "system_command".to_string(),
                arguments: args.to_string(),
            },
        }
    }

    #[test]
    fn glob_pattern_wraps_bare_queries_but_leaves_globs_alone() {
        // The LLM often omits wildcards (e.g. "jks" for "files ending in jks"),
        // so a bare query must become a substring/extension match rather than
        // a literal exact-name match that silently finds nothing.
        assert_eq!(as_glob_pattern("jks"), "*jks*");
        assert_eq!(as_glob_pattern(".jks"), "*.jks");
        assert_eq!(as_glob_pattern("*.jks"), "*.jks");
        assert_eq!(as_glob_pattern("spaceghost.jks"), "*spaceghost.jks*");
    }

    #[test]
    fn expand_path_resolves_tilde_using_home_env_var() {
        // HOME on Unix/macOS, USERPROFILE on Windows - mirrors expand_path itself.
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .expect("HOME or USERPROFILE must be set to run this test");
        assert_eq!(expand_path("~"), home);
        assert_eq!(
            expand_path("~/Git/androStore"),
            format!("{}/Git/androStore", home)
        );
        // Non-tilde paths must pass through untouched.
        assert_eq!(expand_path("/tmp/foo"), "/tmp/foo");
    }

    #[tokio::test]
    #[cfg(unix)]
    // search_file shells out to GNU/BSD `find -iname`, which doesn't exist on
    // Windows (there is no equivalent flag-for-flag substitute), so this
    // command is Unix-only in the current implementation. Tracked as a real
    // gap for the CI matrix's Windows runner, not something to fake pass.
    async fn search_file_finds_match_with_bare_non_glob_query() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("spaceghost.jks");
        std::fs::write(&file_path, b"keystore-bytes").unwrap();

        let tool = SystemCommandTool::new();
        let result = tool
            .execute(&call(json!({
                "command": "search_file",
                "path": dir.path().to_str().unwrap(),
                "query": "jks"
            })))
            .await
            .unwrap();

        assert!(
            result.result.contains("spaceghost.jks"),
            "expected match for bare 'jks' query, got: {}",
            result.result
        );
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn search_file_returns_matches_even_when_find_hits_a_permission_denied_subdir() {
        // Reproduces the real failure: `find` exits with status 1 whenever it
        // can't descend into some subdirectory (e.g. searching from `/` hits
        // `/root`, `/etc/sssd`, etc.), even though it already printed real
        // matches to stdout. The old code discarded stdout on any non-zero
        // exit status, so a genuine match got reported as "no files found".
        // Unix-only: relies on POSIX permission bits (`find` behaves
        // differently on Windows and there's no direct equivalent).
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let match_path = dir.path().join("spaceghost.jks");
        std::fs::write(&match_path, b"keystore-bytes").unwrap();

        let locked_dir = dir.path().join("locked");
        std::fs::create_dir(&locked_dir).unwrap();
        std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o000)).unwrap();

        let tool = SystemCommandTool::new();
        let result = tool
            .execute(&call(json!({
                "command": "search_file",
                "path": dir.path().to_str().unwrap(),
                "query": "*.jks"
            })))
            .await
            .unwrap();

        // Restore permissions so tempdir can clean itself up.
        std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            result.result.contains("spaceghost.jks"),
            "expected the real match to survive a permission-denied sibling directory, got: {}",
            result.result
        );
    }

    #[tokio::test]
    async fn search_file_reports_missing_path_clearly() {
        let tool = SystemCommandTool::new();
        let result = tool
            .execute(&call(json!({
                "command": "search_file",
                "path": "/this/path/does/not/exist",
                "query": "*.jks"
            })))
            .await
            .unwrap();

        assert!(result.result.contains("does not exist"));
    }

    #[test]
    fn check_path_or_parent_exists_allows_new_files_but_not_bad_directories() {
        let dir = tempfile::tempdir().unwrap();
        let existing_file = dir.path().join("existing.txt");
        std::fs::write(&existing_file, b"x").unwrap();

        // The path itself already exists.
        assert!(check_path_or_parent_exists(existing_file.to_str().unwrap()).is_none());

        // A not-yet-created file is fine as long as its parent dir exists
        // (opening a new file in an editor is normal usage).
        let new_file = dir.path().join("new_notes.txt");
        assert!(check_path_or_parent_exists(new_file.to_str().unwrap()).is_none());

        // Neither the path nor its parent exists.
        let bad_path = dir.path().join("no_such_dir").join("file.txt");
        assert!(check_path_or_parent_exists(bad_path.to_str().unwrap()).is_some());
    }

    #[tokio::test]
    async fn ping_host_reaches_loopback() {
        let tool = SystemCommandTool::new();
        let result = tool
            .execute(&call(json!({
                "command": "ping_host",
                "query": "127.0.0.1"
            })))
            .await
            .unwrap();

        assert!(
            result.result.contains("127.0.0.1"),
            "expected ping output for loopback, got: {}",
            result.result
        );
    }

    #[tokio::test]
    async fn ping_host_requires_query() {
        let tool = SystemCommandTool::new();
        let result = tool
            .execute(&call(json!({ "command": "ping_host" })))
            .await
            .unwrap();

        assert!(result.result.contains("'query' parameter"));
    }
}
