use crate::api::llama_server::types::Config;
use crate::api::pageindex::build::{
    build_index, check_llama_reachable, derive_title_from_filename,
};
use crate::api::pageindex::storage::PageIndexStorage;
use crate::api::pageindex::websocket::PageIndexWebSocketState;
use actix_multipart::Multipart;
use actix_web::{post, web, HttpResponse};
use futures_util::TryStreamExt;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[post("/api/pageindex/documents/upload")]
pub async fn upload_document(
    mut payload: Multipart,
    storage: web::Data<Arc<PageIndexStorage>>,
    ws_state: web::Data<PageIndexWebSocketState>,
    llama_config: web::Data<Arc<Mutex<Config>>>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    // Parse multipart form data completely before spawning (Multipart is !Send).
    // Accepts one or more "files" fields, mirroring the ChromaDB upload convention.
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition
            .as_ref()
            .and_then(|cd| cd.get_name())
            .unwrap_or("");

        if field_name == "files" || field_name == "file" {
            let filename = content_disposition
                .as_ref()
                .and_then(|cd| cd.get_filename())
                .unwrap_or("unknown")
                .to_string();

            let mut bytes = Vec::new();
            while let Ok(Some(chunk)) = field.try_next().await {
                bytes.extend_from_slice(&chunk);
            }

            if !bytes.is_empty() {
                files.push((filename, bytes));
            }
        }
    }

    if files.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "At least one 'files' field with a PDF is required"
        })));
    }

    let mut valid_files: Vec<(String, Vec<u8>)> = Vec::new();
    let mut skipped_files: Vec<String> = Vec::new();

    for (filename, bytes) in files {
        if filename.to_lowercase().ends_with(".pdf") {
            valid_files.push((filename, bytes));
        } else {
            skipped_files.push(filename);
        }
    }

    if valid_files.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Only PDF files are supported"
        })));
    }

    // Derive llama base URL / model name exactly as the agent chat endpoints do
    // (src/api/agent/service/chat.rs)
    let model_name = {
        let guard = llama_config.lock().unwrap();
        guard.hf_model.clone()
    };
    let (llama_host, llama_port) = {
        let guard = llama_config.lock().unwrap();
        (
            guard
                .host
                .clone()
                .unwrap_or_else(|| "localhost".to_string()),
            guard.port.unwrap_or(8090),
        )
    };
    let host_for_url = if llama_host == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        llama_host
    };
    let llama_base_url = format!("http://{}:{}", host_for_url, llama_port);

    // Fail fast if the local LLM isn't reachable, rather than accepting the
    // upload(s) and letting every LLM call in the background pipeline fail
    // deep inside build_index for each file.
    if let Err(e) = check_llama_reachable(&llama_base_url).await {
        return Ok(HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "success": false,
            "error": e
        })));
    }

    // Relay progress messages to the WebSocket broadcast (mirrors chromadb's upload handler).
    // Shared across all files in this request since they all report to the same log stream.
    let (tx, mut rx) = mpsc::channel::<serde_json::Value>(100);
    let ws_relay = ws_state.get_ref().clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            ws_relay.broadcast(msg);
        }
    });

    let mut accepted: Vec<serde_json::Value> = Vec::new();

    for (filename, file_data) in valid_files {
        let id = uuid::Uuid::new_v4().to_string();
        let title = derive_title_from_filename(&filename);

        if let Err(e) = storage.insert_pending(&id, &filename, &title).await {
            println!(
                "⚠️ Failed to record PageIndex document '{}': {}",
                filename, e
            );
            continue;
        }

        let storage_for_build = storage.get_ref().clone();
        let ws_for_build = ws_state.get_ref().clone();
        let llama_base_url_for_build = llama_base_url.clone();
        let model_name_for_build = model_name.clone();
        let tx_for_build = tx.clone();
        let id_for_build = id.clone();
        let filename_for_build = filename.clone();

        tokio::spawn(async move {
            build_index(
                id_for_build,
                filename_for_build,
                file_data,
                storage_for_build,
                ws_for_build,
                llama_base_url_for_build,
                model_name_for_build,
                tx_for_build,
            )
            .await;
        });

        accepted.push(serde_json::json!({ "id": id, "filename": filename }));
    }

    if accepted.is_empty() {
        return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": "Failed to record any of the uploaded documents"
        })));
    }

    let mut message = format!("Indexing started for {} file(s)", accepted.len());
    if !skipped_files.is_empty() {
        message.push_str(&format!(
            ". Skipped non-PDF files: {}",
            skipped_files.join(", ")
        ));
    }

    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "success": true,
        "message": message,
        "documents": accepted
    })))
}
