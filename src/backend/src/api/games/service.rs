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
