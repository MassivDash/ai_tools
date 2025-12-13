use crate::cli::config::create_dotenv::create_dotenv_frontend;
use crate::cli::config::get_config::Config;
use crate::cli::pre_run::npm::checks::NPM;
use crate::cli::utils::terminal::{
    dev_info, do_chromadb_log, do_front_log, do_server_log, step, success, warning,
};
use ctrlc::set_handler;
use std::io::Read;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

/// Start the development server
/// The development server will start the actix backend server and the astro frontend server
/// The development server will also check if the port is available for the backend server, and loop until it finds the available port
/// The development server will also clean up the orphaned processes, otherwise cargo watch and node watch will continue to run, blocking the ports.
pub fn start_development(config: Config) {
    // Set the ctrl-c handler to exit the program and clean up orphaned processes
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Check if the port is available for the backend server
    let mut port = config.port.unwrap_or(8080);
    let mut astro_port = config.astro_port.unwrap_or(5431);
    let mut chromadb_port = 8000;

    // Extract port from chroma_address if provided, otherwise use default 8000
    if let Some(ref chroma_addr) = config.chroma_address {
        if let Some(port_str) = chroma_addr.split(':').last() {
            if let Ok(parsed_port) = port_str.parse::<u16>() {
                chromadb_port = parsed_port;
            }
        }
    }

    let mut rust_port_listener = std::net::TcpListener::bind(format!("{}:{}", config.host, port));
    let mut astro_port_listener =
        std::net::TcpListener::bind(format!("{}:{}", config.host, astro_port));
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

    while astro_port_listener.is_err() {
        warning(format!("Port {} is not available", astro_port).as_str());
        astro_port += 1;
        astro_port_listener =
            std::net::TcpListener::bind(format!("{}:{}", config.host, astro_port));
    }

    // kill the listener
    drop(astro_port_listener);

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
    let mut chromadb_server = Command::new(NPM)
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

    // Wait for ChromaDB server to be ready
    let mut buffer_chromadb = [0; 1024];
    let mut stdout_chromadb = chromadb_server.stdout.take().unwrap();
    loop {
        let n = stdout_chromadb.read(&mut buffer_chromadb).unwrap();
        if n == 0 {
            break;
        }
        let s = String::from_utf8_lossy(&buffer_chromadb[..n]);

        do_chromadb_log(&s);

        // ChromaDB typically outputs "Running Chroma" or "Uvicorn running" when ready
        // Also check for port binding messages
        if s.contains("Running Chroma")
            || s.contains("Chroma is running")
            || s.contains("Uvicorn running")
            || s.contains(format!(":{}", chromadb_port).as_str())
        {
            success("ChromaDB server is ready");
            break;
        }
    }

    // Crate the host env for astro to call the actix backend server
    create_dotenv_frontend(
        &format!("http://{}:{}/api/", config.host, port),
        "./src/frontend/.env",
    );

    // Start the backend development server
    step("Start the actix backend development server");
    let mut cargo_watch = Command::new("cargo")
        .current_dir("./src/backend")
        .arg("watch")
        .arg("-w")
        .arg("./src")
        .arg("-x")
        .arg({
            let mut cmd = format!(
                "run -- --host={} --port={} --chroma_address={}",
                config.host, port, chromadb_address
            );
            if let Some(ref domain) = config.cookie_domain {
                cmd.push_str(&format!(" --cookie_domain={}", domain));
            }
            cmd
        })
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start backend development server");

    // Wait for the backend development server to start

    let mut buffer_rust = [0; 1024];
    let mut stdout_rust = cargo_watch.stdout.take().unwrap();
    loop {
        let n = stdout_rust.read(&mut buffer_rust).unwrap();
        if n == 0 {
            break;
        }
        let s = String::from_utf8_lossy(&buffer_rust[..n]);

        do_server_log(&s);
        if s.contains("Actix server has started 🚀") {
            dev_info(&config.host, &port);
            success("Actix server is running, starting the frontend development server");
            break;
        }
    }

    // Start the frontend development server
    step("Starting astro frontend development server");

    let mut node_watch = Command::new(NPM)
        .arg("run")
        .arg("start")
        .arg("--")
        .arg("--port")
        .arg(astro_port.to_string())
        .stdout(std::process::Stdio::piped())
        .current_dir("./src/frontend")
        .spawn()
        .expect("Failed to start frontend development server");

    // Watch the std output of astro bundle if std will have "ready" then open the browser to the development server

    let mut buffer_node = [0; 1024];
    let mut stdout_node = node_watch.stdout.take().unwrap();
    loop {
        let n = stdout_node.read(&mut buffer_node).unwrap();
        if n == 0 {
            break;
        }
        let s = String::from_utf8_lossy(&buffer_node[..n]);

        do_front_log(&s);

        if s.contains("ready") {
            success("Astro is ready, opening the browser");

            let browser = Command::new("open")
                .arg(format!("http://localhost:{}", astro_port))
                .spawn();

            match browser {
                Ok(_) => break,
                Err(err) => {
                    println!("Failed to execute command: {}", err);
                    println!("Are You a Ci Secret Agent ?");

                    // Handle the error here
                }
            }

            break;
        }
    }

    // We want to transmit the stdout_node, stdout_rust, and stdout_chromadb as long as watchers are present
    // Use a simple approach: read from each in sequence with small delays
    loop {
        // Read from ChromaDB
        match stdout_chromadb.read(&mut buffer_chromadb) {
            Ok(n) if n > 0 => {
                let s = String::from_utf8_lossy(&buffer_chromadb[..n]);
                do_chromadb_log(&s);
            }
            _ => {}
        }

        // Read from Rust backend
        match stdout_rust.read(&mut buffer_rust) {
            Ok(n) if n > 0 => {
                let s = String::from_utf8_lossy(&buffer_rust[..n]);
                do_server_log(&s);
            }
            _ => {}
        }

        // Read from Astro frontend
        match stdout_node.read(&mut buffer_node) {
            Ok(n) if n > 0 => {
                let s = String::from_utf8_lossy(&buffer_node[..n]);
                do_front_log(&s);
            }
            _ => {}
        }

        // Small delay to prevent busy waiting
        sleep(Duration::from_millis(50));

        // Check if all processes have exited
        if chromadb_server.try_wait().is_ok()
            && cargo_watch.try_wait().is_ok()
            && node_watch.try_wait().is_ok()
        {
            break;
        }
    }

    // Clean up section for orphaned processes, otherwise cargo watch and node watch will continue to run blocking the ports
    while running.load(Ordering::SeqCst) {
        sleep(Duration::from_millis(100));
    }
    step("Cleaning up orphaned processes");

    chromadb_server
        .kill()
        .expect("Failed to kill chromadb process");
    cargo_watch
        .kill()
        .expect("Failed to kill cargo-watch process");
    node_watch
        .kill()
        .expect("Failed to kill node-watch process");

    step("Exiting");

    std::process::exit(0);
}
