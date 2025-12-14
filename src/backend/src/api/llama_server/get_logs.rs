use actix_web::{get, web, HttpResponse, Result as ActixResult};
use serde::Serialize;

use crate::api::llama_server::types::{LogBuffer, LogSource};

#[derive(Serialize, Debug)]
pub struct LogLine {
    pub timestamp: u64,
    pub line: String,
    pub source: String,
}

#[derive(Serialize, Debug)]
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
