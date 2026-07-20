use crate::cli::config::create_dotenv::create_dotenv_frontend;
use crate::cli::config::get_config::{get_config, Config, ASTROX_TOML};
use crate::cli::config::toml::read_toml;
use crate::cli::pre_run::npm::checks::NPM;
use crate::cli::utils::terminal::{do_chromadb_log, step, success, warning};
use ctrlc::set_handler;
use std::io::{BufRead, BufReader};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

/// Start the production server
/// The production server will start the actix backend server
/// The production server will also bundle the frontend
pub fn start_production(config: Config) {
    // Bundle the frontend and wait for the process to finish
    // if the astro build is set to true
    // start the build process

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Check if the port is available for the backend server
    let mut port = config.port.unwrap_or(8080);
    let mut chromadb_port = 8000;

    // Extract port from chroma_address if provided, otherwise use default 8000
    if let Some(ref chroma_addr) = config.chroma_address {
        if let Some(port_str) = chroma_addr.split(':').next_back() {
            if let Ok(parsed_port) = port_str.parse::<u16>() {
                chromadb_port = parsed_port;
            }
        }
    }

    let mut rust_port_listener = std::net::TcpListener::bind(format!("{}:{}", config.host, port));
    let mut chromadb_port_listener =
        std::net::TcpListener::bind(format!("{}:{}", config.host, chromadb_port));

    // Loop until you find the port that is available
    while rust_port_listener.is_err() {
        warning(format!("Port {} is not available", port).as_str());
        port += 1;
        rust_port_listener = std::net::TcpListener::bind(format!("{}:{}", config.host, port));
    }

    // kill the listener
    drop(rust_port_listener);

    while chromadb_port_listener.is_err() {
        warning(format!("ChromaDB port {} is not available", chromadb_port).as_str());
        chromadb_port += 1;
        chromadb_port_listener =
            std::net::TcpListener::bind(format!("{}:{}", config.host, chromadb_port));
    }

    // kill the listener
    drop(chromadb_port_listener);

    // Build the final ChromaDB address using the actual port (may have been incremented)
    let chromadb_address = format!("http://{}:{}", config.host, chromadb_port);

    // Start ChromaDB server using chroma run command directly
    step("Starting ChromaDB server");
    let mut chromadb_cmd = Command::new(NPM);
    #[cfg(unix)]
    chromadb_cmd.process_group(0);

    let mut chromadb_server = chromadb_cmd
        .current_dir("./src/chromadb")
        .arg("start")
        .arg("--")
        .arg("--host")
        .arg(&config.host)
        .arg("--port")
        .arg(chromadb_port.to_string())
        .arg("--path")
        .arg("./database")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start ChromaDB server");

    // Wait for ChromaDB server to be ready and set up continuous reading
    let stdout_chromadb = chromadb_server.stdout.take().unwrap();
    let chromadb_ready = Arc::new(AtomicBool::new(false));
    let chromadb_ready_clone = chromadb_ready.clone();
    let chromadb_port_clone = chromadb_port;

    // Spawn thread to read ChromaDB logs continuously
    let chromadb_handle = thread::spawn(move || {
        let mut reader = BufReader::new(stdout_chromadb);
        let mut buf = Vec::new();
        while let Ok(bytes_read) = reader.read_until(b'\n', &mut buf) {
            if bytes_read == 0 {
                break;
            }
            let line = String::from_utf8_lossy(&buf)
                .trim_end_matches(&['\r', '\n'][..])
                .to_string();
            buf.clear();
            if !line.trim().is_empty() {
                do_chromadb_log(&format!("{}\n", line));

                // Check if ChromaDB is ready
                if !chromadb_ready_clone.load(Ordering::SeqCst)
                    && (line.contains("Running Chroma")
                        || line.contains("Chroma is running")
                        || line.contains("Uvicorn running")
                        || line.contains(format!(":{}", chromadb_port_clone).as_str()))
                {
                    chromadb_ready_clone.store(true, Ordering::SeqCst);
                    success("ChromaDB server is ready");
                }
            }
        }
    });

    // Wait for ChromaDB to be ready before starting backend
    while !chromadb_ready.load(Ordering::SeqCst) {
        sleep(Duration::from_millis(100));
    }

    if config.prod_astro_build {
        // take production build url from config
        create_dotenv_frontend(
            &config.public_keys.public_api_url,
            config.public_keys.public_llama_url.as_deref(),
            "./src/frontend/.env",
        );

        step("Bundling the frontend");

        let bundle = Command::new(NPM)
            .arg("run")
            .arg("build")
            .current_dir("./src/frontend")
            .spawn()
            .expect("Failed to bundle the frontend")
            .wait()
            .expect("Failed to bundle the frontend");

        match bundle.success() {
            true => step("Frontend bundled successfully"),
            false => panic!("Failed to bundle the frontend"),
        }
    }

    // Start the backend production server
    step("Starting cargo backend production server");

    let mut cargo_server = {
        let mut cargo_command = Command::new("cargo");
        #[cfg(unix)]
        cargo_command.process_group(0);

        cargo_command
            .current_dir("./src/backend")
            .arg("run")
            .arg("--release")
            .arg("--")
            .arg(format!("--host={}", config.host))
            .arg(format!("--port={}", port))
            .arg(format!("--env={}", config.env))
            .arg(format!("--cors_url={}", config.cors_url))
            .arg(format!("--chroma_address={}", chromadb_address));

        if let Some(ref domain) = config.cookie_domain {
            cargo_command.arg(format!("--cookie_domain={}", domain));
        }

        if let Some(ref llama_host) = config.llama_host {
            cargo_command.arg(format!("--llama_host={}", llama_host));
        }
        if let Some(ref llama_port) = config.llama_port {
            cargo_command.arg(format!("--llama_port={}", llama_port));
        }

        cargo_command
            .spawn()
            .expect("Failed to start backend production server")
    };

    // Main loop: keep the process alive and monitor all services
    // All log reading is handled by the spawned threads above
    loop {
        sleep(Duration::from_millis(100));

        if !running.load(Ordering::SeqCst) {
            break;
        }

        // Check if ChromaDB exited (critical, exit if it does)
        if let Ok(Some(_)) = chromadb_server.try_wait() {
            step("ChromaDB server has exited");
            break;
        }

        // Check if threads have finished (streams closed)
        if chromadb_handle.is_finished() {
            break;
        }

        // Check if backend exited and needs restart
        if let Ok(Some(status)) = cargo_server.try_wait() {
            if !status.success() && running.load(Ordering::SeqCst) {
                step("Backend production server exited, restarting...");

                let mut cargo_command = Command::new("cargo");
                #[cfg(unix)]
                cargo_command.process_group(0);

                cargo_command
                    .current_dir("./src/backend")
                    .arg("run")
                    .arg("--release")
                    .arg("--")
                    .arg(format!("--host={}", config.host))
                    .arg(format!("--port={}", port))
                    .arg(format!("--env={}", config.env))
                    .arg(format!("--cors_url={}", config.cors_url))
                    .arg(format!("--chroma_address={}", chromadb_address));

                if let Some(ref domain) = config.cookie_domain {
                    cargo_command.arg(format!("--cookie_domain={}", domain));
                }

                if let Some(ref llama_host) = config.llama_host {
                    cargo_command.arg(format!("--llama_host={}", llama_host));
                }
                if let Some(ref llama_port) = config.llama_port {
                    cargo_command.arg(format!("--llama_port={}", llama_port));
                }

                cargo_server = cargo_command
                    .spawn()
                    .expect("Failed to restart backend production server");
            } else {
                break;
            }
        }
    }

    step("Cleaning up orphaned processes");

    #[cfg(unix)]
    {
        // Kill the entire process groups (including any spawned children) using negative PID
        let _ = Command::new("kill")
            .arg("-9")
            .arg("--")
            .arg(format!("-{}", chromadb_server.id()))
            .status();
        let _ = Command::new("kill")
            .arg("-9")
            .arg("--")
            .arg(format!("-{}", cargo_server.id()))
            .status();
        let _ = Command::new("pkill").arg("-9").arg("llama-server").status();
        // cargo run's child (the actual backend binary) can escape cargo's
        // process group, so the group kill above may not reach it and it stays
        // bound to the port. Kill it directly by binary path as a fallback.
        let _ = Command::new("pkill")
            .arg("-9")
            .arg("-f")
            .arg("target/release/backend")
            .status();
        let _ = chromadb_server.wait();
        let _ = cargo_server.wait();
    }
    #[cfg(not(unix))]
    {
        let _ = chromadb_server.kill();
        let _ = cargo_server.kill();
        let _ = Command::new("taskkill")
            .arg("/F")
            .arg("/IM")
            .arg("llama-server.exe")
            .status();

        let _ = chromadb_server.wait();
        let _ = cargo_server.wait();
    }

    step("Exiting");

    std::process::exit(0);
}

pub fn execute_serve() {
    let config = read_toml(&ASTROX_TOML.to_string());
    match config {
        Ok(mut config) => {
            config.env = "prod".to_string();
            start_production(config);
        }
        Err(_) => {
            let mut config = get_config(&vec![]);
            config.env = "prod".to_string();
            start_production(config);
        }
    }
}
