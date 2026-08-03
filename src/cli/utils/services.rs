use crate::cli::pre_run::npm::checks::NPM;
use crate::cli::utils::logs::{handle_chromadb_line, stream_lines};
use crate::cli::utils::terminal::step;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

/// A running ChromaDB server together with the thread streaming its logs and
/// the flag raised once the server reports being ready.
pub struct ChromadbService {
    pub child: Child,
    pub logs: JoinHandle<()>,
    pub ready: Arc<AtomicBool>,
}

/// Spawn the ChromaDB server in its own process group and stream its logs.
/// Shared by the development and the production server.
pub fn start_chromadb(host: &str, port: u16) -> ChromadbService {
    step("Starting ChromaDB server");

    let mut chromadb_cmd = Command::new(NPM);
    #[cfg(unix)]
    chromadb_cmd.process_group(0);

    let mut child = chromadb_cmd
        .current_dir("./src/chromadb")
        .arg("start")
        .arg("--")
        .arg("--host")
        .arg(host)
        .arg("--port")
        .arg(port.to_string())
        .arg("--path")
        .arg("./database")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start ChromaDB server");

    let stdout = child.stdout.take().unwrap();
    let ready = Arc::new(AtomicBool::new(false));
    let ready_clone = ready.clone();

    let logs = thread::spawn(move || {
        stream_lines(stdout, |line| {
            handle_chromadb_line(line, port, &ready_clone)
        });
    });

    ChromadbService { child, logs, ready }
}

/// Clean up the orphaned processes left behind by the servers.
///
/// On unix the whole process group of every child is killed, plus any stray
/// llama-server and any backend binary that escaped its group. On other systems
/// the children are killed directly. Every child is reaped afterwards.
pub fn terminate_services(children: &mut [&mut Child], backend_binary: &str) {
    #[cfg(unix)]
    {
        // Kill the entire process groups (including any spawned children) using negative PID
        for child in children.iter() {
            let _ = Command::new("kill")
                .arg("-9")
                .arg("--")
                .arg(format!("-{}", child.id()))
                .status();
        }

        let _ = Command::new("pkill").arg("-9").arg("llama-server").status();
        // cargo run's child (the actual backend binary) can escape the cargo
        // process group, so the group kill above may not reach it and it stays
        // bound to the port. Kill it directly by binary path as a fallback.
        let _ = Command::new("pkill")
            .arg("-9")
            .arg("-f")
            .arg(backend_binary)
            .status();
    }

    #[cfg(not(unix))]
    {
        let _ = backend_binary;

        for child in children.iter_mut() {
            let _ = child.kill();
        }

        let _ = Command::new("taskkill")
            .arg("/F")
            .arg("/IM")
            .arg("llama-server.exe")
            .status();
    }

    for child in children.iter_mut() {
        let _ = child.wait();
    }
}
