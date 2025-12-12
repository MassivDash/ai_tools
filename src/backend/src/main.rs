use std::env;

use actix_files::{Files, NamedFile};
use actix_rt::System;
use actix_web::dev::{fn_service, ServiceRequest, ServiceResponse};
use actix_web::middleware::{NormalizePath, TrailingSlash};
use actix_web::{middleware, web, App, HttpServer};

mod api;
mod args;
mod auth;
mod cors;
mod markdown_utils;
mod session;
mod ssr_routes;

use crate::api::llama_server::get_config::get_llama_config;
use crate::api::llama_server::get_logs::get_llama_logs;
use crate::api::llama_server::get_models::get_llama_models;
use crate::api::llama_server::get_status::get_llama_server_status;
use crate::api::llama_server::post_config::post_update_config;
use crate::api::llama_server::post_start::post_start_llama_server;
use crate::api::llama_server::post_stop::post_stop_llama_server;
use crate::api::llama_server::types::{Config, LogBuffer, ServerState, ServerStateHandle};
use crate::api::url_to_markdown::post::convert_url_to_markdown;
use crate::args::collect_args::collect_args;
use crate::auth::auth_middleware::Authentication;
use crate::cors::get_cors_options::get_cors_options;
use crate::session::flash_messages::set_up_flash_messages;
use crate::ssr_routes::login::login_form;
use crate::ssr_routes::post_login::post_login;
use std::process::Child;
use std::sync::{Arc, Mutex};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = collect_args(env::args().collect());
    let host = args.host;
    let port = args.port.parse::<u16>().unwrap();
    let cors_url = args.cors_url;
    let cookie_domain = args.cookie_domain;

    // Shared state for llama server process
    let llama_process: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let llama_config: Arc<Mutex<Config>> = Arc::new(Mutex::new(Config::default()));
    let llama_logs: LogBuffer = Arc::new(Mutex::new(std::collections::VecDeque::new()));
    let llama_server_state: ServerStateHandle = Arc::new(Mutex::new(ServerState { is_ready: false }));

    // Set up the actix server
    let llama_process_data = llama_process.clone();
    let llama_config_data = llama_config.clone();
    let llama_logs_data = llama_logs.clone();
    let llama_server_state_data = llama_server_state.clone();
    let server = HttpServer::new(move || {
        let env = args.env.to_string();
        let cors = get_cors_options(env, cors_url.clone()); //Prod CORS URL address, for dev run the cors is set to *
        let auth_routes: Vec<String> = vec!["/auth/*".to_string()]; // Routes that require authentication

        // The services and wrappers are loaded from the last to first
        // Ensure all the wrappers are after routes and handlers
        App::new()
            .app_data(web::Data::new(llama_process_data.clone()))
            .app_data(web::Data::new(llama_config_data.clone()))
            .app_data(web::Data::new(llama_logs_data.clone()))
            .app_data(web::Data::new(llama_server_state_data.clone()))
            .wrap(cors)
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(post_login))
            .service(convert_url_to_markdown)
            .service(get_llama_server_status)
            .service(get_llama_models)
            .service(get_llama_config)
            .service(get_llama_logs)
            .service(post_start_llama_server)
            .service(post_stop_llama_server)
            .service(post_update_config)
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
            .wrap(Authentication {
                routes: auth_routes,
            })
            .wrap(session::session_middleware::session_middleware(
                cookie_domain.clone(),
            ))
            .wrap(set_up_flash_messages())
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
