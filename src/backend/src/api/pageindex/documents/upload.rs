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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MockLlm, MockLlmConfig};
    use actix_web::{test, App};

    const BOUNDARY: &str = "----------------pageindextest";

    /// Build a `multipart/form-data` body. Parts with a filename are sent as file
    /// uploads; the rest as plain fields.
    fn multipart_body(parts: &[(&str, Option<&str>, &[u8])]) -> Vec<u8> {
        let mut body = Vec::new();
        for (name, filename, content) in parts {
            body.extend_from_slice(format!("--{}\r\n", BOUNDARY).as_bytes());
            match filename {
                Some(filename) => body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n\
                         Content-Type: application/octet-stream\r\n\r\n",
                        name, filename
                    )
                    .as_bytes(),
                ),
                None => body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", name).as_bytes(),
                ),
            }
            body.extend_from_slice(content);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{}--\r\n", BOUNDARY).as_bytes());
        body
    }

    fn llama_config(base_url: &str) -> Arc<Mutex<Config>> {
        let url = url::Url::parse(base_url).unwrap();
        Arc::new(Mutex::new(Config {
            host: Some(url.host_str().unwrap().to_string()),
            port: Some(url.port().unwrap()),
            ..Config::default()
        }))
    }

    /// A llama config pointing at a port that always refuses connections.
    fn unreachable_llama_config() -> Arc<Mutex<Config>> {
        Arc::new(Mutex::new(Config {
            host: Some("127.0.0.1".to_string()),
            port: Some(1),
            ..Config::default()
        }))
    }

    async fn upload(
        storage: Arc<PageIndexStorage>,
        llama_config: Arc<Mutex<Config>>,
        parts: &[(&str, Option<&str>, &[u8])],
    ) -> (u16, serde_json::Value) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(storage))
                .app_data(web::Data::new(PageIndexWebSocketState::new()))
                .app_data(web::Data::new(llama_config))
                .service(upload_document),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/pageindex/documents/upload")
            .insert_header((
                "content-type",
                format!("multipart/form-data; boundary={}", BOUNDARY),
            ))
            .set_payload(multipart_body(parts))
            .to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status().as_u16();
        (status, test::read_body_json(resp).await)
    }

    async fn empty_storage() -> Arc<PageIndexStorage> {
        Arc::new(PageIndexStorage::new(":memory:").await.unwrap())
    }

    /// Remove the `./public/pageindex/{id}` directories the background indexing
    /// task creates, once it has had a chance to run.
    async fn cleanup(ids: &[String]) {
        for _ in 0..50 {
            if ids
                .iter()
                .all(|id| std::path::Path::new("./public/pageindex").join(id).exists())
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        for id in ids {
            let _ = std::fs::remove_dir_all(std::path::Path::new("./public/pageindex").join(id));
        }
    }

    fn accepted_ids(body: &serde_json::Value) -> Vec<String> {
        body["documents"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["id"].as_str().unwrap().to_string())
            .collect()
    }

    #[actix_web::test]
    async fn test_upload_without_any_file_field_is_rejected() {
        let (status, body) = upload(
            empty_storage().await,
            unreachable_llama_config(),
            &[("notes", None, b"some text")],
        )
        .await;

        assert_eq!(status, 400);
        assert_eq!(body["success"], false);
        assert_eq!(
            body["error"],
            "At least one 'files' field with a PDF is required"
        );
    }

    #[actix_web::test]
    async fn test_upload_with_an_empty_file_is_rejected() {
        // A zero-byte part is dropped, leaving nothing to index.
        let (status, body) = upload(
            empty_storage().await,
            unreachable_llama_config(),
            &[("files", Some("empty.pdf"), b"")],
        )
        .await;

        assert_eq!(status, 400);
        assert_eq!(
            body["error"],
            "At least one 'files' field with a PDF is required"
        );
    }

    #[actix_web::test]
    async fn test_upload_of_only_non_pdf_files_is_rejected() {
        let (status, body) = upload(
            empty_storage().await,
            unreachable_llama_config(),
            &[
                ("files", Some("notes.txt"), b"plain text"),
                ("files", Some("data.csv"), b"a,b"),
            ],
        )
        .await;

        assert_eq!(status, 400);
        assert_eq!(body["success"], false);
        assert_eq!(body["error"], "Only PDF files are supported");
    }

    #[actix_web::test]
    async fn test_upload_fails_fast_when_the_local_llm_is_unreachable() {
        let storage = empty_storage().await;

        let (status, body) = upload(
            storage.clone(),
            unreachable_llama_config(),
            &[("files", Some("book.pdf"), b"%PDF-1.4 fake")],
        )
        .await;

        assert_eq!(status, 503);
        assert_eq!(body["success"], false);
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("Could not reach the local LLM server"));

        // Nothing was recorded, so the user can simply retry.
        assert!(storage.list_documents().await.unwrap().is_empty());
    }

    #[actix_web::test]
    async fn test_upload_accepts_a_pdf_and_records_it_as_processing() {
        let llm = MockLlm::start(MockLlmConfig::replying("ok")).await;
        let storage = empty_storage().await;

        let (status, body) = upload(
            storage.clone(),
            llama_config(&llm.base_url),
            &[("files", Some("clean-code.pdf"), b"%PDF-1.4 fake")],
        )
        .await;

        assert_eq!(status, 202);
        assert_eq!(body["success"], true);
        assert_eq!(body["message"], "Indexing started for 1 file(s)");

        let docs = body["documents"].as_array().unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0]["filename"], "clean-code.pdf");

        let stored = storage.list_documents().await.unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].filename, "clean-code.pdf");
        // The title is derived from the filename by the upload handler.
        assert_eq!(stored[0].title, "Clean Code");

        cleanup(&accepted_ids(&body)).await;
        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_upload_accepts_the_singular_file_field_name_too() {
        let llm = MockLlm::start(MockLlmConfig::replying("ok")).await;
        let storage = empty_storage().await;

        let (status, body) = upload(
            storage.clone(),
            llama_config(&llm.base_url),
            &[("file", Some("solo.PDF"), b"%PDF-1.4 fake")],
        )
        .await;

        // The extension check is case-insensitive.
        assert_eq!(status, 202);
        assert_eq!(storage.list_documents().await.unwrap().len(), 1);

        cleanup(&accepted_ids(&body)).await;
        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_upload_reports_skipped_non_pdf_files_alongside_the_accepted_ones() {
        let llm = MockLlm::start(MockLlmConfig::replying("ok")).await;
        let storage = empty_storage().await;

        let (status, body) = upload(
            storage.clone(),
            llama_config(&llm.base_url),
            &[
                ("files", Some("a.pdf"), b"%PDF-1.4 fake"),
                ("files", Some("notes.txt"), b"plain text"),
                ("files", Some("b.pdf"), b"%PDF-1.4 fake"),
            ],
        )
        .await;

        assert_eq!(status, 202);
        let message = body["message"].as_str().unwrap();
        assert!(message.starts_with("Indexing started for 2 file(s)"));
        assert!(message.contains("Skipped non-PDF files: notes.txt"));
        assert_eq!(body["documents"].as_array().unwrap().len(), 2);
        assert_eq!(storage.list_documents().await.unwrap().len(), 2);

        cleanup(&accepted_ids(&body)).await;
        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_upload_returns_500_when_no_document_can_be_recorded() {
        let llm = MockLlm::start(MockLlmConfig::replying("ok")).await;
        let storage = PageIndexStorage::new(":memory:").await.unwrap();
        storage.drop_table_for_tests().await;

        let (status, body) = upload(
            Arc::new(storage),
            llama_config(&llm.base_url),
            &[("files", Some("book.pdf"), b"%PDF-1.4 fake")],
        )
        .await;

        assert_eq!(status, 500);
        assert_eq!(body["success"], false);
        assert_eq!(
            body["error"],
            "Failed to record any of the uploaded documents"
        );

        llm.stop().await;
    }

    #[actix_web::test]
    async fn test_upload_rejects_an_llm_that_answers_with_an_error() {
        let mut config = MockLlmConfig::replying("ok");
        config.props_status = 500;
        let llm = MockLlm::start(config).await;
        let storage = empty_storage().await;

        let (status, body) = upload(
            storage.clone(),
            llama_config(&llm.base_url),
            &[("files", Some("book.pdf"), b"%PDF-1.4 fake")],
        )
        .await;

        assert_eq!(status, 503);
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("responded with an error"));
        assert!(storage.list_documents().await.unwrap().is_empty());

        llm.stop().await;
    }
}
