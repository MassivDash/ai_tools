use crate::cli::config::create_dotenv::create_dotenv_frontend;
use crate::cli::config::get_config::Config;
use crate::cli::pre_run::npm::checks::NPM;
use crate::cli::utils::terminal::{
    dev_info, do_chromadb_log, do_front_log, do_server_log, step, success, warning,
};
use ctrlc::set_handler;
use std::io::{BufRead, BufReader};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
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
        if let Some(port_str) = chroma_addr.split(':').next_back() {
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

    // Wait for ChromaDB server to be ready and set up continuous reading
    let stdout_chromadb = chromadb_server.stdout.take().unwrap();
    let chromadb_ready = Arc::new(AtomicBool::new(false));
    let chromadb_ready_clone = chromadb_ready.clone();
    let chromadb_port_clone = chromadb_port;

    // Spawn thread to read ChromaDB logs continuously
    let chromadb_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout_chromadb);
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
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
                Err(_) => break,
            }
        }
    });

    // Wait for ChromaDB to be ready before starting backend
    while !chromadb_ready.load(Ordering::SeqCst) {
        sleep(Duration::from_millis(100));
    }

    // Crate the host env for astro to call the actix backend server
    create_dotenv_frontend(
        &format!("http://{}:{}/api/", config.host, port),
        config.public_keys.public_llama_url.as_deref(),
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
            if let Some(ref llama_host) = config.llama_host {
                cmd.push_str(&format!(" --llama_host={}", llama_host));
            }
            if let Some(ref llama_port) = config.llama_port {
                cmd.push_str(&format!(" --llama_port={}", llama_port));
            }
            cmd
        })
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start backend development server");

    // Wait for the backend development server to start and set up continuous reading
    let stdout_rust = cargo_watch.stdout.take().unwrap();
    let rust_ready = Arc::new(AtomicBool::new(false));
    let rust_ready_clone = rust_ready.clone();
    let host_clone = config.host.clone();
    let port_clone = port;

    // Spawn thread to read Rust backend logs continuously
    let rust_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout_rust);
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    if !line.trim().is_empty() {
                        do_server_log(&format!("{}\n", line));

                        // Check if Actix server is ready
                        if !rust_ready_clone.load(Ordering::SeqCst)
                            && line.contains("Actix server has started 🚀")
                        {
                            rust_ready_clone.store(true, Ordering::SeqCst);
                            dev_info(&host_clone, &port_clone);
                            success(
                                "Actix server is running, starting the frontend development server",
                            );
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Wait for Rust backend to be ready before starting frontend
    while !rust_ready.load(Ordering::SeqCst) {
        sleep(Duration::from_millis(100));
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
    let stdout_node = node_watch.stdout.take().unwrap();
    let astro_ready = Arc::new(AtomicBool::new(false));
    let astro_ready_clone = astro_ready.clone();
    let astro_port_clone = astro_port;

    // Spawn thread to read Astro frontend logs continuously
    let astro_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout_node);
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    if !line.trim().is_empty() {
                        do_front_log(&format!("{}\n", line));

                        // Check if Astro is ready and open browser
                        if !astro_ready_clone.load(Ordering::SeqCst) && line.contains("ready") {
                            astro_ready_clone.store(true, Ordering::SeqCst);
                            success("Astro is ready, opening the browser");

                            let browser = Command::new("open")
                                .arg(format!("http://localhost:{}", astro_port_clone))
                                .spawn();

                            if let Err(err) = browser {
                                println!("Failed to execute command: {}", err);
                                println!("Are You a Ci Secret Agent ?");
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Main loop: keep the process alive and monitor all three services
    // All log reading is handled by the spawned threads above
    loop {
        sleep(Duration::from_millis(100));

        // Check if all processes have exited
        if chromadb_server.try_wait().is_ok()
            && cargo_watch.try_wait().is_ok()
            && node_watch.try_wait().is_ok()
        {
            break;
        }

        // Check if threads have finished (streams closed)
        if chromadb_handle.is_finished() && rust_handle.is_finished() && astro_handle.is_finished()
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
