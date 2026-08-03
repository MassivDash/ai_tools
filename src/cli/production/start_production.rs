use crate::cli::config::create_dotenv::create_dotenv_frontend;
use crate::cli::config::get_config::{get_prod_config, Config};
use crate::cli::pre_run::npm::checks::NPM;
use crate::cli::production::build_production::{BuildRunner, RealBuildRunner};
use crate::cli::utils::logs::wait_until_ready;
use crate::cli::utils::ports::{bind_available_port, chromadb_port_from_config};
use crate::cli::utils::server_args::BackendArgs;
use crate::cli::utils::services::{start_chromadb, terminate_services};
use crate::cli::utils::terminal::step;
use ctrlc::set_handler;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

/// What the production monitor loop should do on the current tick.
#[derive(Debug, PartialEq, Eq)]
pub enum MonitorAction {
    /// Nothing happened, keep watching the services.
    KeepRunning,
    /// Stop the server.
    Stop,
    /// ChromaDB is gone, report it and stop the server.
    ChromadbExited,
    /// The backend died on its own while we are still running, bring it back.
    RestartBackend,
}

/// Decide what to do with the production services on the current tick.
///
/// `backend_exit` is `None` while the backend is still alive, otherwise it
/// carries whether the backend exited successfully.
pub fn next_monitor_action(
    running: bool,
    chromadb_exited: bool,
    chromadb_logs_finished: bool,
    backend_exit: Option<bool>,
) -> MonitorAction {
    if !running {
        return MonitorAction::Stop;
    }

    // ChromaDB is critical, without it the backend is useless
    if chromadb_exited {
        return MonitorAction::ChromadbExited;
    }

    if chromadb_logs_finished {
        return MonitorAction::Stop;
    }

    match backend_exit {
        // The backend crashed while we are still up, restart it
        Some(false) => MonitorAction::RestartBackend,
        // A clean backend exit means we are done
        Some(true) => MonitorAction::Stop,
        None => MonitorAction::KeepRunning,
    }
}

/// Spawn the release build of the actix backend in its own process group.
fn spawn_backend(
    config: &Config,
    port: u16,
    chromadb_address: &str,
    failure_message: &str,
) -> Child {
    let args = BackendArgs {
        host: &config.host,
        port,
        env: Some(&config.env),
        cors_url: Some(&config.cors_url),
        chroma_address: chromadb_address,
        cookie_domain: config.cookie_domain.as_deref(),
        llama_host: config.llama_host.as_deref(),
        llama_port: config.llama_port,
    }
    .to_args();

    let mut cargo_command = Command::new("cargo");
    #[cfg(unix)]
    cargo_command.process_group(0);

    cargo_command
        .current_dir("./src/backend")
        .arg("run")
        .arg("--release")
        .arg("--")
        .args(args)
        .spawn()
        .expect(failure_message)
}

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

    // Check if the ports are available, the listeners are held until every port
    // has been picked so that two services can never land on the same port
    let (port, rust_port_listener) =
        bind_available_port(&config.host, config.port.unwrap_or(8080), "Port");
    let (chromadb_port, chromadb_port_listener) = bind_available_port(
        &config.host,
        chromadb_port_from_config(config.chroma_address.as_deref()),
        "ChromaDB port",
    );

    // kill the listeners
    drop(rust_port_listener);
    drop(chromadb_port_listener);

    // Build the final ChromaDB address using the actual port (may have been incremented)
    let chromadb_address = format!("http://{}:{}", config.host, chromadb_port);

    // Start ChromaDB server and wait for it to be ready before starting backend
    let mut chromadb = start_chromadb(&config.host, chromadb_port);
    wait_until_ready(&chromadb.ready);

    if config.prod_astro_build {
        // take production build url from config
        create_dotenv_frontend(
            &config.public_keys.public_api_url,
            config.public_keys.public_llama_url.as_deref(),
            "./src/frontend/.env",
        );

        step("Bundling the frontend");

        match RealBuildRunner.run(
            NPM,
            &["run", "build"],
            "./src/frontend",
            "Failed to bundle the frontend",
        ) {
            true => step("Frontend bundled successfully"),
            false => panic!("Failed to bundle the frontend"),
        }
    }

    // Start the backend production server
    step("Starting cargo backend production server");

    let mut cargo_server = spawn_backend(
        &config,
        port,
        &chromadb_address,
        "Failed to start backend production server",
    );

    // Main loop: keep the process alive and monitor all services
    // All log reading is handled by the spawned threads above
    loop {
        sleep(Duration::from_millis(100));

        let chromadb_exited = matches!(chromadb.child.try_wait(), Ok(Some(_)));
        let backend_exit = match cargo_server.try_wait() {
            Ok(Some(status)) => Some(status.success()),
            _ => None,
        };

        match next_monitor_action(
            running.load(Ordering::SeqCst),
            chromadb_exited,
            chromadb.logs.is_finished(),
            backend_exit,
        ) {
            MonitorAction::KeepRunning => continue,
            MonitorAction::Stop => break,
            MonitorAction::ChromadbExited => {
                step("ChromaDB server has exited");
                break;
            }
            MonitorAction::RestartBackend => {
                step("Backend production server exited, restarting...");

                cargo_server = spawn_backend(
                    &config,
                    port,
                    &chromadb_address,
                    "Failed to restart backend production server",
                );
            }
        }
    }

    step("Cleaning up orphaned processes");

    terminate_services(
        &mut [&mut chromadb.child, &mut cargo_server],
        "target/release/backend",
    );

    step("Exiting");

    std::process::exit(0);
}

pub fn execute_serve() {
    start_production(get_prod_config());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_keeps_running_while_everything_is_alive() {
        assert_eq!(
            next_monitor_action(true, false, false, None),
            MonitorAction::KeepRunning
        );
    }

    #[test]
    fn test_monitor_stops_on_ctrl_c() {
        assert_eq!(
            next_monitor_action(false, false, false, None),
            MonitorAction::Stop
        );
    }

    #[test]
    fn test_monitor_reports_a_dead_chromadb() {
        assert_eq!(
            next_monitor_action(true, true, false, None),
            MonitorAction::ChromadbExited
        );
    }

    #[test]
    fn test_ctrl_c_wins_over_a_dead_chromadb() {
        // On ctrl-c we stop quietly, without reporting ChromaDB
        assert_eq!(
            next_monitor_action(false, true, false, None),
            MonitorAction::Stop
        );
    }

    #[test]
    fn test_monitor_stops_when_the_chromadb_logs_close() {
        assert_eq!(
            next_monitor_action(true, false, true, None),
            MonitorAction::Stop
        );
    }

    #[test]
    fn test_monitor_restarts_a_crashed_backend() {
        assert_eq!(
            next_monitor_action(true, false, false, Some(false)),
            MonitorAction::RestartBackend
        );
    }

    #[test]
    fn test_monitor_stops_on_a_clean_backend_exit() {
        assert_eq!(
            next_monitor_action(true, false, false, Some(true)),
            MonitorAction::Stop
        );
    }

    #[test]
    fn test_a_crashed_backend_does_not_restart_after_ctrl_c() {
        assert_eq!(
            next_monitor_action(false, false, false, Some(false)),
            MonitorAction::Stop
        );
    }
}
