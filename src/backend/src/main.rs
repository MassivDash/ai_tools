use std::env;

use actix_files::{Files, NamedFile};
use actix_rt::System;
use actix_web::dev::{fn_service, ServiceRequest, ServiceResponse};
use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::{middleware, web, App, HttpServer};

mod api;
mod args;
mod cors;
mod markdown_utils;
mod services;
mod utils;

use dotenv::dotenv;

use crate::api::agent::core::types::AgentConfig;
use crate::api::agent::memory::sqlite_memory::SqliteConversationMemory;
use crate::api::agent::service::config::AgentConfigHandle;
use crate::api::agent::service::websocket::{agent_websocket, AgentWebSocketState};
use crate::api::agent::testing::storage::TestingStorage;
use crate::api::chromadb::config::types::ChromaDBConfig;
use crate::api::default_configs::DefaultConfigsStorage;
use crate::api::llama_server::types::{
    Config, LogBuffer, ProcessHandle, ServerState, ServerStateHandle,
};
use crate::api::llama_server::websocket::{logs_websocket, status_websocket, WebSocketState};
use crate::api::model_notes::ModelNotesStorage;
use crate::args::collect_args::collect_args;
use crate::cors::get_cors_options::get_cors_options;
use crate::services::agent::configure_agent_services;
use crate::services::chromadb::configure_chromadb_services;
use crate::services::converters::configure_converter_services;
use crate::services::llama_server::configure_llama_server_services;
use crate::services::model_notes::configure_model_notes_services;

use std::process::Child;
use std::sync::{Arc, Mutex};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let args = collect_args(env::args().collect());
    let host = args.host;
    let port = args.port.parse::<u16>().unwrap();
    let cors_url = args.cors_url;

    // Get chroma_address from args or use default
    let chroma_address = args
        .chroma_address
        .unwrap_or_else(|| "http://localhost:8000".to_string());
    println!("🔗 ChromaDB address: {}", chroma_address);

    // SQLite-based conversation storage (persists user/assistant messages)
    let sqlite_memory: Arc<SqliteConversationMemory> = Arc::new(
        SqliteConversationMemory::new("./data/conversations.db")
            .await
            .expect("Failed to initialize SQLite conversation memory"),
    );

    // SQLite-based model notes storage
    let model_notes_storage: Arc<ModelNotesStorage> = Arc::new(
        ModelNotesStorage::new("./data/conversations.db")
            .await
            .expect("Failed to initialize model notes storage"),
    );

    // SQLite-based default configs storage (separate from model notes)
    let default_configs_storage: Arc<DefaultConfigsStorage> = Arc::new(
        DefaultConfigsStorage::new("./data/conversations.db")
            .await
            .expect("Failed to initialize default configs storage"),
    );

    // SQLite-based testing storage
    use sqlx::sqlite::SqlitePoolOptions;
    let testing_pool = SqlitePoolOptions::new()
        .connect("./data/conversations.db")
        .await
        .expect("Failed to connect to testing database");

    let testing_storage = TestingStorage::new(testing_pool)
        .await
        .expect("Failed to initialize testing storage");
    // TestingStorage implements Clone (contains SqlitePool), so we don't need Arc wrapper for Data

    // Initialize llama config with default from storage or fallback to hardcoded
    let mut llama_config_init = Config::default();
    if let Ok(Some(default_config)) = default_configs_storage.get_llama_default().await {
        llama_config_init.hf_model = default_config.hf_model.clone();
        println!(
            "✅ Using default Llama model from config: {}",
            default_config.hf_model
        );
    } else {
        println!(
            "ℹ️  No default Llama model set, using hardcoded: {}",
            llama_config_init.hf_model
        );
    }
    let llama_config: Arc<Mutex<Config>> = Arc::new(Mutex::new(llama_config_init));

    // Shared state for llama server process
    let llama_process: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let llama_logs: LogBuffer = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let llama_server_state: ServerStateHandle =
        Arc::new(Mutex::new(ServerState { is_ready: false }));

    // Initialize ChromaDB config with default from storage or fallback to hardcoded
    let mut chromadb_config_init = ChromaDBConfig::default();
    if let Ok(Some(default_config)) = default_configs_storage.get_chromadb_default().await {
        chromadb_config_init.embedding_model = default_config.embedding_model.clone();
        chromadb_config_init.query_model = default_config.embedding_model.clone();
        println!(
            "✅ Using default ChromaDB model from config: {}",
            default_config.embedding_model
        );
    } else {
        println!(
            "ℹ️  No default ChromaDB model set, using hardcoded: {}",
            chromadb_config_init.embedding_model
        );
    }
    let chromadb_config: Arc<Mutex<ChromaDBConfig>> = Arc::new(Mutex::new(chromadb_config_init));

    // Shared state for agent config
    let agent_config: AgentConfigHandle = Arc::new(Mutex::new(AgentConfig::default()));

    // Create Agent WebSocket state for real-time agent updates
    let agent_ws_state = Arc::new(AgentWebSocketState::new());

    // Create WebSocket state ONCE before the server (shared across all workers)
    let ws_state = Arc::new(WebSocketState::new(
        web::Data::new(llama_logs.clone()),
        web::Data::new(llama_process.clone()),
        web::Data::new(llama_server_state.clone()),
    ));

    // Start status polling task ONCE (outside of HttpServer::new)
    let ws_state_status = ws_state.clone();
    let llama_process_status = llama_process.clone();
    let llama_server_state_status = llama_server_state.clone();
    actix_rt::spawn(async move {
        use tokio::time::{interval, Duration};
        let mut interval = interval(Duration::from_secs(2));

        loop {
            interval.tick().await;

            let process_handle: ProcessHandle = llama_process_status.clone();
            let state_handle: ServerStateHandle = llama_server_state_status.clone();

            let is_active = {
                let mut process_guard = process_handle.lock().unwrap();
                if let Some(ref mut child) = *process_guard {
                    match child.try_wait() {
                        Ok(Some(_)) => {
                            drop(process_guard);
                            let mut p = process_handle.lock().unwrap();
                            *p = None;
                            false
                        }
                        Ok(None) => true,
                        Err(_) => false,
                    }
                } else {
                    false
                }
            };

            let is_ready = {
                let state_guard = state_handle.lock().unwrap();
                state_guard.is_ready
            };

            // Check port
            let port_check = tokio::net::TcpStream::connect("127.0.0.1:8080")
                .await
                .is_ok();

            let active = is_active && (is_ready || port_check);

            ws_state_status.broadcast_status(active, 8080);
        }
    });

    // Set up the actix server
    let llama_process_data = llama_process.clone();
    let llama_config_data = llama_config.clone();
    let llama_logs_data = llama_logs.clone();
    let llama_server_state_data = llama_server_state.clone();
    let ws_state_data = ws_state.clone();
    let agent_ws_state_data = agent_ws_state.clone();
    let chroma_address_data = web::Data::new(chroma_address.clone());
    let chromadb_config_data = chromadb_config.clone();
    let agent_config_data = agent_config.clone();
    let sqlite_memory_data = web::Data::new(sqlite_memory.clone());
    let model_notes_storage_data = web::Data::new(model_notes_storage.clone());
    let default_configs_storage_data = web::Data::new(default_configs_storage.clone());
    let server = HttpServer::new(move || {
        let env = args.env.to_string();
        let cors = get_cors_options(env, cors_url.clone()); //Prod CORS URL address, for dev run the cors is set to *

        // The services and wrappers are loaded from the last to first
        // Ensure all the wrappers are after routes and handlers
        App::new()
            .app_data(web::Data::new(llama_process_data.clone()))
            .app_data(web::Data::new(llama_config_data.clone()))
            .app_data(web::Data::new(llama_logs_data.clone()))
            .app_data(web::Data::new(llama_server_state_data.clone()))
            .app_data(web::Data::new(ws_state_data.clone()))
            .app_data(web::Data::new(agent_ws_state_data.clone()))
            .app_data(chroma_address_data.clone())
            .app_data(web::Data::new(chromadb_config_data.clone()))
            .app_data(web::Data::new(agent_config_data.clone()))
            .app_data(sqlite_memory_data.clone())
            .app_data(model_notes_storage_data.clone())
            .app_data(default_configs_storage_data.clone())
            .app_data(web::Data::new(testing_storage.clone()))
            .wrap(cors)
            .route("/api/llama-server/logs/ws", web::get().to(logs_websocket))
            .route(
                "/api/llama-server/status/ws",
                web::get().to(status_websocket),
            )
            .route("/api/agent/stream/ws", web::get().to(agent_websocket))
            .configure(configure_converter_services)
            .configure(configure_llama_server_services)
            .configure(configure_chromadb_services)
            .configure(configure_agent_services)
            .configure(configure_model_notes_services)
            .service(
                Files::new("/", "../frontend/dist/")
                    .prefer_utf8(true)
                    .index_file("index.html")
                    .default_handler(fn_service(|req: ServiceRequest| async {
                        let (req, _) = req.into_parts();
                        let file = NamedFile::open_async("../frontend/dist/404.html").await?;
                        let res = file.into_response(&req);
                        Ok(ServiceResponse::new(req, res))
                    })),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(NormalizePath::new(TrailingSlash::Trim)) // Add this line to handle trailing slashes\
    })
    .bind((host, port))?;

    let server = server.run();

    System::current().arbiter().spawn(async {
        println!("Actix server has started 🚀");
    });

    server.await
}
