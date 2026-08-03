use crate::api::games::types::{GameChatRequest, GameStreamEvent};
use crate::api::llama_server::types::Config;
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Streaming game chat completion endpoint
#[post("/api/games/chat/stream")]
pub async fn game_chat_stream(
    req: web::Json<GameChatRequest>,
    llama_config: web::Data<Arc<Mutex<Config>>>,
) -> ActixResult<HttpResponse> {
    // Construct Llama URL from config
    let (llama_host, llama_port) = {
        let llama_config_guard = llama_config.lock().unwrap();
        (
            llama_config_guard
                .host
                .clone()
                .unwrap_or_else(|| "localhost".to_string()),
            llama_config_guard.port.unwrap_or(8090),
        )
    };

    let host_for_url = if llama_host == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        llama_host
    };
    let llama_base_url = format!("http://{}:{}", host_for_url, llama_port);
    let llama_url = format!("{}/v1/chat/completions", llama_base_url);

    let client = Client::new();

    // Prepare messages: System prompt + User message
    // Note: In a real game, you might want to maintain history/context.
    // For V1 "stateless host", we might just send the system prompt + last user input,
    // or the frontend can send a few recent messages if needed.
    // Based on the plan, we are just sending "message" and "system_prompt".
    // We'll treat "message" as the latest user input.
    // If the user wants history, they should bundle it or we'd need to store it.
    // For now, let's assume a stateless turn-based interaction where the prompt contains necessary context or just reacts.

    let messages = vec![
        json!({ "role": "system", "content": req.system_prompt }),
        json!({ "role": "user", "content": req.message }),
    ];

    let request_body = json!({
        "messages": messages,
        "stream": true,
        "temperature": 0.7, // Add config for this later if needed
    });

    // Create channel for streaming events
    let (tx, rx) = mpsc::channel::<Result<GameStreamEvent, anyhow::Error>>(100);

    // Spawn background task to stream from Llama
    actix_rt::spawn(async move {
        // Send request to Llama
        let response = match client.post(&llama_url).json(&request_body).send().await {
            Ok(res) => res,
            Err(e) => {
                let _ = tx
                    .send(Ok(GameStreamEvent::Error {
                        message: e.to_string(),
                    }))
                    .await;
                return;
            }
        };

        if !response.status().is_success() {
            let _ = tx
                .send(Ok(GameStreamEvent::Error {
                    message: format!("Llama server error: {}", response.status()),
                }))
                .await;
            return;
        }

        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    // bytes is bytes::Bytes
                    let chunk_str = String::from_utf8_lossy(&bytes);
                    for line in chunk_str.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                let _ = tx.send(Ok(GameStreamEvent::Done)).await;
                                break;
                            }

                            // Check if data is valid JSON
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(choices) = json.get("choices") {
                                    if let Some(choice) = choices.get(0) {
                                        if let Some(delta) = choice.get("delta") {
                                            if let Some(content) = delta.get("content") {
                                                if let Some(text) = content.as_str() {
                                                    if !text.is_empty() {
                                                        let _ = tx
                                                            .send(Ok(GameStreamEvent::TextChunk {
                                                                text: text.to_string(),
                                                            }))
                                                            .await;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(Ok(GameStreamEvent::Error {
                            message: e.to_string(),
                        }))
                        .await;
                    break;
                }
            }
        }
    });

    // Convert channel to SSE stream
    let stream = tokio_stream::wrappers::ReceiverStream::new(rx).map(
        move |event_result| -> Result<web::Bytes, actix_web::Error> {
            match event_result {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                    Ok(web::Bytes::from(format!("data: {}\n\n", json)))
                }
                Err(e) => {
                    let error_event = GameStreamEvent::Error {
                        message: e.to_string(),
                    };
                    let json =
                        serde_json::to_string(&error_event).unwrap_or_else(|_| "{}".to_string());
                    Ok(web::Bytes::from(format!("data: {}\n\n", json)))
                }
            }
        },
    );

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .append_header(("Cache-Control", "no-cache"))
        .append_header(("Connection", "keep-alive"))
        .streaming(stream))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::dev::ServerHandle;
    use actix_web::http::StatusCode;
    use actix_web::{test, App, HttpServer};

    /// Starts a throwaway HTTP server on a free loopback port that answers
    /// `/v1/chat/completions` with `body`. The handler talks to whatever host
    /// and port the llama config points at, so this stands in for llama.cpp
    /// without any network access.
    fn spawn_fake_llama(status: u16, body: &'static str) -> (u16, ServerHandle) {
        let server = HttpServer::new(move || {
            App::new().route(
                "/v1/chat/completions",
                web::post().to(move || async move {
                    HttpResponse::build(StatusCode::from_u16(status).unwrap())
                        .content_type("text/event-stream")
                        .body(body)
                }),
            )
        })
        .workers(1)
        .bind(("127.0.0.1", 0))
        .expect("Failed to bind the fake llama server");

        let port = server.addrs()[0].port();
        let server = server.run();
        let handle = server.handle();
        actix_rt::spawn(server);

        (port, handle)
    }

    fn config_for(host: &str, port: u16) -> Arc<Mutex<Config>> {
        Arc::new(Mutex::new(Config {
            host: Some(host.to_string()),
            port: Some(port),
            ..Default::default()
        }))
    }

    /// Collects the `data: {...}` frames of an SSE body into events.
    fn parse_sse(body: &[u8]) -> Vec<GameStreamEvent> {
        String::from_utf8_lossy(body)
            .split("\n\n")
            .filter_map(|frame| frame.trim().strip_prefix("data: ").map(str::to_string))
            .map(|json| {
                serde_json::from_str::<GameStreamEvent>(&json)
                    .unwrap_or_else(|e| panic!("Failed to parse event {}: {}", json, e))
            })
            .collect()
    }

    async fn call_stream(llama_config: Arc<Mutex<Config>>) -> (u16, Vec<GameStreamEvent>) {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(llama_config))
                .service(game_chat_stream),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/games/chat/stream")
            .set_json(json!({
                "message": "let's play",
                "system_prompt": "you are the host"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status().as_u16();
        let body = test::read_body(resp).await;

        (status, parse_sse(&body))
    }

    #[actix_web::test]
    async fn test_stream_forwards_content_deltas_and_the_done_marker() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n\
                   data: {\"choices\":[{\"delta\":{\"content\":\" world\"}}]}\n\n\
                   data: [DONE]\n\n";
        let (port, handle) = spawn_fake_llama(200, sse);

        let (status, events) = call_stream(config_for("127.0.0.1", port)).await;

        assert_eq!(status, 200);
        assert_eq!(events.len(), 3, "got {:?}", events);
        match &events[0] {
            GameStreamEvent::TextChunk { text } => assert_eq!(text, "Hello"),
            other => panic!("Expected a text chunk, got {:?}", other),
        }
        match &events[1] {
            GameStreamEvent::TextChunk { text } => assert_eq!(text, " world"),
            other => panic!("Expected a text chunk, got {:?}", other),
        }
        assert!(
            matches!(events[2], GameStreamEvent::Done),
            "got {:?}",
            events[2]
        );

        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn test_stream_skips_empty_malformed_and_contentless_frames() {
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"\"}}]}\n\n\
                   data: {\"choices\":[{\"delta\":{}}]}\n\n\
                   data: {\"choices\":[{}]}\n\n\
                   data: {\"choices\":[]}\n\n\
                   data: {\"no_choices\":true}\n\n\
                   data: {\"choices\":[{\"delta\":{\"content\":42}}]}\n\n\
                   data: not-json-at-all\n\n\
                   : a comment line that is not a data frame\n\n\
                   data: {\"choices\":[{\"delta\":{\"content\":\"only me\"}}]}\n\n\
                   data: [DONE]\n\n";
        let (port, handle) = spawn_fake_llama(200, sse);

        let (status, events) = call_stream(config_for("127.0.0.1", port)).await;

        assert_eq!(status, 200);
        assert_eq!(events.len(), 2, "got {:?}", events);
        match &events[0] {
            GameStreamEvent::TextChunk { text } => assert_eq!(text, "only me"),
            other => panic!("Expected a text chunk, got {:?}", other),
        }
        assert!(matches!(events[1], GameStreamEvent::Done));

        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn test_stream_reports_a_llama_error_status() {
        let (port, handle) = spawn_fake_llama(503, "unavailable");

        // "0.0.0.0" must be rewritten to the loopback address before the request
        let (status, events) = call_stream(config_for("0.0.0.0", port)).await;

        assert_eq!(status, 200, "the SSE channel itself still opens");
        assert_eq!(events.len(), 1, "got {:?}", events);
        match &events[0] {
            GameStreamEvent::Error { message } => assert_eq!(
                message, "Llama server error: 503 Service Unavailable",
                "unexpected message: {}",
                message
            ),
            other => panic!("Expected an error, got {:?}", other),
        }

        handle.stop(true).await;
    }

    #[actix_web::test]
    async fn test_stream_reports_an_unreachable_llama_server() {
        // Port 1 on loopback has nothing listening, so the request fails fast.
        let (status, events) = call_stream(config_for("127.0.0.1", 1)).await;

        assert_eq!(status, 200);
        assert_eq!(events.len(), 1, "got {:?}", events);
        match &events[0] {
            GameStreamEvent::Error { message } => {
                assert!(!message.is_empty(), "the connection error is forwarded")
            }
            other => panic!("Expected an error, got {:?}", other),
        }
    }

    #[actix_web::test]
    async fn test_stream_defaults_to_localhost_when_no_host_is_configured() {
        // A closed port keeps this deterministic while still exercising the
        // "localhost" default for a config without a host.
        let llama_config = Arc::new(Mutex::new(Config {
            host: None,
            port: Some(1),
            ..Default::default()
        }));

        let (status, events) = call_stream(llama_config).await;

        assert_eq!(status, 200);
        assert_eq!(events.len(), 1, "got {:?}", events);
        match &events[0] {
            GameStreamEvent::Error { message } => assert!(
                message.contains("localhost:1"),
                "the default host should appear in the connection error: {}",
                message
            ),
            other => panic!("Expected an error, got {:?}", other),
        }
    }

    #[actix_web::test]
    async fn test_stream_rejects_a_body_without_a_system_prompt() {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(config_for("127.0.0.1", 1)))
                .service(game_chat_stream),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/api/games/chat/stream")
            .set_json(json!({ "message": "let's play" }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 400);
    }
}
