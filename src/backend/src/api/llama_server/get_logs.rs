use actix_web::{get, web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};

use crate::api::llama_server::types::{LogBuffer, LogSource};

#[derive(Serialize, Deserialize, Debug)]
pub struct LogLine {
    pub timestamp: u64,
    pub line: String,
    pub source: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LogsResponse {
    pub logs: Vec<LogLine>,
}

#[get("/api/llama-server/logs")]
pub async fn get_llama_logs(log_buffer: web::Data<LogBuffer>) -> ActixResult<HttpResponse> {
    let buffer = log_buffer.lock().unwrap();
    let logs: Vec<LogLine> = buffer
        .iter()
        .map(|entry| LogLine {
            timestamp: entry.timestamp,
            line: entry.line.clone(),
            source: match entry.source {
                LogSource::Stdout => "stdout".to_string(),
                LogSource::Stderr => "stderr".to_string(),
            },
        })
        .collect();

    Ok(HttpResponse::Ok().json(LogsResponse { logs }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::llama_server::types::{LogBuffer, LogEntry, LogSource};
    use actix_web::{test, web, App};
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[actix_web::test]
    async fn test_get_llama_logs_empty() {
        let log_buffer: LogBuffer = Arc::new(Mutex::new(VecDeque::new()));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(log_buffer))
                .service(get_llama_logs),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/llama-server/logs")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body: LogsResponse = test::read_body_json(resp).await;
        assert!(body.logs.is_empty());
    }

    #[actix_web::test]
    async fn test_get_llama_logs_with_entries() {
        let mut buffer = VecDeque::new();
        buffer.push_back(LogEntry {
            timestamp: 1234567890,
            line: "Test log line".to_string(),
            source: LogSource::Stdout,
        });
        buffer.push_back(LogEntry {
            timestamp: 1234567891,
            line: "Error log line".to_string(),
            source: LogSource::Stderr,
        });

        let log_buffer: LogBuffer = Arc::new(Mutex::new(buffer));

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(log_buffer))
                .service(get_llama_logs),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/api/llama-server/logs")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert!(resp.status().is_success());
        let body: LogsResponse = test::read_body_json(resp).await;
        assert_eq!(body.logs.len(), 2);
        assert_eq!(body.logs[0].line, "Test log line");
        assert_eq!(body.logs[0].source, "stdout");
        assert_eq!(body.logs[1].line, "Error log line");
        assert_eq!(body.logs[1].source, "stderr");
    }
}
