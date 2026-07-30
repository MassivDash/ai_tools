//! PageIndex ingestion pipeline: turns an uploaded PDF into a hierarchical
//! table-of-contents tree with an LLM-generated summary per node.

use crate::api::agent::core::types::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, MessageContent, MessageRole,
};
use crate::api::pageindex::outline::{extract_bookmark_tree, fill_page_ends};
use crate::api::pageindex::storage::PageIndexStorage;
use crate::api::pageindex::types::PageIndexNode;
use crate::api::pageindex::websocket::PageIndexWebSocketState;
use crate::api::shared::pdf::{count_pdf_pages, extract_pdf_text};
use futures::future::BoxFuture;
use futures::FutureExt;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Number of pages fed to the LLM per structure-inference call.
const STRUCTURE_WINDOW_PAGES: usize = 15;
/// Cap on how much section text we send to the LLM for summarization.
const MAX_SECTION_TEXT_CHARS: usize = 6000;

/// Timeout for the pre-flight local LLM reachability check.
const LLM_REACHABILITY_TIMEOUT_SECS: u64 = 3;

/// Pre-flight check used by the upload handler to fail fast when the local LLM
/// server is switched off, instead of accepting the upload and letting every
/// subsequent LLM call in the pipeline fail deep inside a background task.
/// Mirrors the reachability check already used by `GET /api/agent/model-capabilities`
/// (hitting the llama.cpp server's `/props` endpoint), but with an explicit short
/// timeout so an unreachable server fails the request quickly.
pub async fn check_llama_reachable(llama_base_url: &str) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(LLM_REACHABILITY_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let props_url = format!("{}/props", llama_base_url);
    match client.get(&props_url).send().await {
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) => Err(format!(
            "Local LLM server responded with an error ({}). Make sure it's fully started before uploading books.",
            response.status()
        )),
        Err(_) => Err(format!(
            "Could not reach the local LLM server at {}. Start it from the Llama Server page before uploading books \u{2014} PageIndex needs it to build the table of contents and section summaries.",
            llama_base_url
        )),
    }
}

/// Derive a human-readable title from an uploaded filename, e.g.
/// `clean-code_a-handbook.pdf` -> `Clean Code A Handbook`.
pub(crate) fn derive_title_from_filename(filename: &str) -> String {
    let stem = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    stem.replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn count_nodes(nodes: &[PageIndexNode]) -> u32 {
    nodes.iter().map(|n| 1 + count_nodes(&n.children)).sum()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }
    let mut end = max_chars;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

/// Send a progress message through the channel; if the receiver has been dropped,
/// fall back to broadcasting directly on the WebSocket state.
async fn broadcast(
    progress_tx: &mpsc::Sender<Value>,
    ws: &PageIndexWebSocketState,
    document_id: &str,
    status: &str,
    message: &str,
    success: Option<bool>,
) {
    let mut val = json!({
        "status": status,
        "message": message,
        "document_id": document_id,
    });
    if let Some(s) = success {
        val["success"] = json!(s);
    }
    if progress_tx.send(val.clone()).await.is_err() {
        ws.broadcast(val);
    }
}

/// Call the LLM with a single-turn user prompt and return the assistant's text content.
async fn call_llm(
    client: &Client,
    llama_url: &str,
    model: &str,
    prompt: &str,
    max_tokens: u32,
) -> Result<String, String> {
    let request = ChatCompletionRequest {
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: MessageContent::Text(prompt.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }],
        model: model.to_string(),
        temperature: Some(0.2),
        max_tokens: Some(max_tokens),
        tools: None,
        tool_choice: None,
        stream: Some(false),
        stream_options: None,
    };

    let response = client
        .post(llama_url)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {}", e))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read LLM response: {}", e))?;

    if !status.is_success() {
        return Err(format!("LLM server error ({}): {}", status, text));
    }

    let parsed: ChatCompletionResponse = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse LLM response: {} (raw: {})", e, text))?;

    let choice = parsed
        .choices
        .first()
        .ok_or_else(|| "LLM response had no choices".to_string())?;

    Ok(choice.message.content.text())
}

#[derive(Debug, Deserialize)]
struct LlmSection {
    title: String,
    page_start: u32,
    #[serde(default = "default_level")]
    level: u8,
}

fn default_level() -> u8 {
    1
}

#[derive(Debug, Deserialize)]
struct LlmSectionsResponse {
    sections: Vec<LlmSection>,
}

/// Best-effort extraction of the first top-level `{...}` JSON object in `text`
/// (models sometimes wrap JSON in prose or markdown code fences).
fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end > start {
        Some(&text[start..=end])
    } else {
        None
    }
}

/// Split `pdftotext` output into per-page text using the form-feed page separators
/// it inserts by default, numbering pages sequentially starting at 1.
fn split_into_pages(full_text: &str) -> Vec<String> {
    full_text.split('\u{c}').map(|p| p.to_string()).collect()
}

/// Group consecutive pages into ~`window_size`-page windows, each rendered as
/// `--- PAGE {n} ---\n{text}` blocks for the structure-inference prompt.
fn build_windows(pages: &[String], window_size: usize) -> Vec<(u32, u32, String)> {
    let mut windows = Vec::new();
    let mut start = 0usize;
    while start < pages.len() {
        let end = (start + window_size).min(pages.len());
        let mut combined = String::new();
        for (offset, page_text) in pages[start..end].iter().enumerate() {
            let page_num = (start + offset + 1) as u32;
            combined.push_str(&format!("--- PAGE {} ---\n{}\n", page_num, page_text.trim()));
        }
        windows.push(((start + 1) as u32, end as u32, combined));
        start = end;
    }
    windows
}

/// LLM-based fallback structure inference, used when a PDF has no usable embedded
/// bookmarks. Processes the document in sliding windows of pages, asking the model
/// to identify chapter/section headings and their starting page numbers.
#[allow(clippy::too_many_arguments)]
async fn infer_structure_via_llm(
    client: &Client,
    llama_url: &str,
    model: &str,
    pdf_bytes: &[u8],
    total_pages: u32,
    progress_tx: &mpsc::Sender<Value>,
    ws: &PageIndexWebSocketState,
    document_id: &str,
) -> Vec<PageIndexNode> {
    let full_text = match extract_pdf_text(pdf_bytes, None) {
        Ok((text, _)) => text,
        Err(e) => {
            broadcast(
                progress_tx,
                ws,
                document_id,
                "error",
                &format!("Failed to extract PDF text for structure inference: {}", e),
                None,
            )
            .await;
            return Vec::new();
        }
    };

    let pages = split_into_pages(&full_text);
    let windows = build_windows(&pages, STRUCTURE_WINDOW_PAGES);
    let mut sections: BTreeMap<u32, (String, u8)> = BTreeMap::new();

    for (i, (win_start, win_end, window_text)) in windows.iter().enumerate() {
        broadcast(
            progress_tx,
            ws,
            document_id,
            "processing",
            &format!(
                "Analyzing structure: pages {}-{} (window {}/{})",
                win_start,
                win_end,
                i + 1,
                windows.len()
            ),
            None,
        )
        .await;

        let prompt = format!(
            "You are analyzing pages from a technical book to reconstruct its table of contents. Below are page-numbered excerpts. Identify chapter and section headings and the page each one starts on. Respond with ONLY valid JSON matching {{\"sections\":[{{\"title\":string,\"page_start\":number,\"level\":1|2}}]}}. Only include clear structural headings, not body text or captions.\n\n{}",
            window_text
        );

        match call_llm(client, llama_url, model, &prompt, 800).await {
            Ok(raw) => {
                if let Some(json_str) = extract_json_object(&raw) {
                    match serde_json::from_str::<LlmSectionsResponse>(json_str) {
                        Ok(parsed) => {
                            for s in parsed.sections {
                                let title = s.title.trim().to_string();
                                if !title.is_empty()
                                    && s.page_start >= 1
                                    && s.page_start <= total_pages
                                {
                                    sections.entry(s.page_start).or_insert((title, s.level));
                                }
                            }
                        }
                        Err(e) => {
                            println!(
                                "⚠️ PageIndex: failed to parse structure window JSON: {} (raw: {})",
                                e, raw
                            );
                        }
                    }
                } else {
                    println!(
                        "⚠️ PageIndex: no JSON object found in structure window response: {}",
                        raw
                    );
                }
            }
            Err(e) => println!("⚠️ PageIndex: structure inference window failed: {}", e),
        }
    }

    let mut roots: Vec<PageIndexNode> = Vec::new();
    let mut counter = 0u32;
    for (page_start, (title, level)) in sections {
        counter += 1;
        let node = PageIndexNode {
            id: format!("n{}", counter),
            title,
            page_start,
            page_end: page_start,
            summary: String::new(),
            children: Vec::new(),
        };

        if level <= 1 || roots.is_empty() {
            roots.push(node);
        } else if let Some(last) = roots.last_mut() {
            last.children.push(node);
        }
    }

    fill_page_ends(&mut roots, total_pages);
    roots
}

struct SummarizeCtx<'a> {
    client: &'a Client,
    llama_url: &'a str,
    model: &'a str,
    pdf_bytes: &'a [u8],
    total_nodes: u32,
    progress: &'a AtomicUsize,
    progress_tx: &'a mpsc::Sender<Value>,
    ws: &'a PageIndexWebSocketState,
    document_id: &'a str,
}

/// Bottom-up recursive summarization: leaves are summarized from their raw extracted
/// text; parents are summarized from their children's already-computed summaries.
/// Boxed because async fns cannot be directly recursive.
fn summarize_nodes<'a>(nodes: &'a mut [PageIndexNode], ctx: &'a SummarizeCtx<'a>) -> BoxFuture<'a, ()> {
    async move {
        for node in nodes.iter_mut() {
            if !node.children.is_empty() {
                summarize_nodes(&mut node.children, ctx).await;
            }

            let summary = if node.children.is_empty() {
                summarize_leaf(node, ctx).await
            } else {
                summarize_parent(node, ctx).await
            };
            node.summary = summary;

            let done = ctx.progress.fetch_add(1, Ordering::SeqCst) + 1;
            broadcast(
                ctx.progress_tx,
                ctx.ws,
                ctx.document_id,
                "processing",
                &format!(
                    "Summarizing section {}/{}: {}",
                    done, ctx.total_nodes, node.title
                ),
                None,
            )
            .await;
        }
    }
    .boxed()
}

async fn summarize_leaf(node: &PageIndexNode, ctx: &SummarizeCtx<'_>) -> String {
    let text = match extract_pdf_text(ctx.pdf_bytes, Some((node.page_start, node.page_end))) {
        Ok((text, _)) => text,
        Err(e) => return format!("(Summary unavailable: failed to extract section text: {})", e),
    };

    let trimmed_text = truncate_chars(text.trim(), MAX_SECTION_TEXT_CHARS);
    if trimmed_text.trim().is_empty() {
        return "(No extractable text was found for this section.)".to_string();
    }

    let prompt = format!(
        "Summarize the following section of a technical book in 2-4 sentences, focused on what topics/concepts it covers so a reader can judge relevance to a question. Section title: {}\n\nText:\n{}",
        node.title, trimmed_text
    );

    match call_llm(ctx.client, ctx.llama_url, ctx.model, &prompt, 300).await {
        Ok(summary) => summary.trim().to_string(),
        Err(e) => {
            println!(
                "⚠️ PageIndex: failed to summarize leaf section '{}': {}",
                node.title, e
            );
            "(Summary unavailable due to an LLM error.)".to_string()
        }
    }
}

async fn summarize_parent(node: &PageIndexNode, ctx: &SummarizeCtx<'_>) -> String {
    let numbered_summaries = node
        .children
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}. {}: {}", i + 1, c.title, c.summary))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "Summarize this chapter based on its sections' summaries. Chapter title: {}\n\nSection summaries:\n{}\n\nWrite a 2-4 sentence chapter-level summary.",
        node.title, numbered_summaries
    );

    match call_llm(ctx.client, ctx.llama_url, ctx.model, &prompt, 300).await {
        Ok(summary) => summary.trim().to_string(),
        Err(e) => {
            println!(
                "⚠️ PageIndex: failed to summarize chapter '{}': {}",
                node.title, e
            );
            "(Summary unavailable due to an LLM error.)".to_string()
        }
    }
}

/// Run the full PageIndex ingestion pipeline for a freshly uploaded PDF.
/// Intended to be `tokio::spawn`ed by the upload handler; reports progress via
/// `progress_tx`/`ws` and always resolves the document to either "ready" or "error"
/// in `storage` rather than panicking.
#[allow(clippy::too_many_arguments)]
pub async fn build_index(
    id: String,
    filename: String,
    pdf_bytes: Vec<u8>,
    storage: Arc<PageIndexStorage>,
    ws: PageIndexWebSocketState,
    llama_base_url: String,
    model_name: String,
    progress_tx: mpsc::Sender<Value>,
) {
    macro_rules! fail {
        ($msg:expr) => {{
            let msg: String = $msg;
            println!("⚠️ PageIndex[{}]: {}", id, msg);
            let _ = storage.mark_error(&id, &msg).await;
            broadcast(&progress_tx, &ws, &id, "error", &msg, Some(false)).await;
            return;
        }};
    }

    let doc_dir = PathBuf::from("./public/pageindex").join(&id);
    if let Err(e) = tokio::fs::create_dir_all(&doc_dir).await {
        fail!(format!("Failed to create storage directory: {}", e));
    }

    let pdf_path = doc_dir.join("source.pdf");
    if let Err(e) = tokio::fs::write(&pdf_path, &pdf_bytes).await {
        fail!(format!("Failed to save source PDF: {}", e));
    }

    broadcast(
        &progress_tx,
        &ws,
        &id,
        "info",
        &format!("Counting pages in {}...", filename),
        None,
    )
    .await;

    let total_pages = match count_pdf_pages(&pdf_path) {
        Ok(p) if p > 0 => p,
        Ok(_) => fail!("PDF appears to have zero pages".to_string()),
        Err(e) => fail!(format!("Failed to count PDF pages: {}", e)),
    };

    let client = Client::new();
    let llama_url = format!("{}/v1/chat/completions", llama_base_url);

    broadcast(
        &progress_tx,
        &ws,
        &id,
        "info",
        "Looking for embedded bookmarks...",
        None,
    )
    .await;

    let mut tree = extract_bookmark_tree(&pdf_path, total_pages).unwrap_or_default();

    if !tree.is_empty() {
        broadcast(
            &progress_tx,
            &ws,
            &id,
            "info",
            &format!(
                "Found {} section(s) from embedded bookmarks",
                count_nodes(&tree)
            ),
            None,
        )
        .await;
    } else {
        broadcast(
            &progress_tx,
            &ws,
            &id,
            "info",
            "No usable bookmarks found, inferring structure with the LLM...",
            None,
        )
        .await;

        tree = infer_structure_via_llm(
            &client,
            &llama_url,
            &model_name,
            &pdf_bytes,
            total_pages,
            &progress_tx,
            &ws,
            &id,
        )
        .await;
    }

    if tree.is_empty() {
        broadcast(
            &progress_tx,
            &ws,
            &id,
            "info",
            "No clear structure found, using a single section for the whole document",
            None,
        )
        .await;

        tree = vec![PageIndexNode {
            id: "n1".to_string(),
            title: derive_title_from_filename(&filename),
            page_start: 1,
            page_end: total_pages,
            summary: String::new(),
            children: Vec::new(),
        }];
    }

    let total_nodes = count_nodes(&tree);
    let progress_counter = AtomicUsize::new(0);
    let ctx = SummarizeCtx {
        client: &client,
        llama_url: &llama_url,
        model: &model_name,
        pdf_bytes: &pdf_bytes,
        total_nodes,
        progress: &progress_counter,
        progress_tx: &progress_tx,
        ws: &ws,
        document_id: &id,
    };

    broadcast(
        &progress_tx,
        &ws,
        &id,
        "processing",
        &format!("Summarizing {} section(s)...", total_nodes),
        None,
    )
    .await;

    summarize_nodes(&mut tree, &ctx).await;

    let tree_json = match serde_json::to_string_pretty(&tree) {
        Ok(j) => j,
        Err(e) => fail!(format!("Failed to serialize index tree: {}", e)),
    };

    let tree_path = doc_dir.join("tree.json");
    if let Err(e) = tokio::fs::write(&tree_path, tree_json).await {
        fail!(format!("Failed to write tree.json: {}", e));
    }

    if let Err(e) = storage.mark_ready(&id, total_pages, total_nodes).await {
        println!(
            "⚠️ PageIndex[{}]: failed to mark document ready in DB: {}",
            id, e
        );
    }

    broadcast(
        &progress_tx,
        &ws,
        &id,
        "completed",
        &format!("Indexed '{}' into {} section(s)", filename, total_nodes),
        Some(true),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_title_from_filename() {
        assert_eq!(
            derive_title_from_filename("clean-code_a-handbook.pdf"),
            "Clean Code A Handbook"
        );
        assert_eq!(derive_title_from_filename("book.pdf"), "Book");
    }

    #[test]
    fn test_split_into_pages_on_form_feed() {
        let text = "page one\u{c}page two\u{c}page three";
        let pages = split_into_pages(text);
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[1], "page two");
    }

    #[test]
    fn test_build_windows_chunks_pages() {
        let pages: Vec<String> = (1..=20).map(|i| format!("text {}", i)).collect();
        let windows = build_windows(&pages, 15);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].0, 1);
        assert_eq!(windows[0].1, 15);
        assert_eq!(windows[1].0, 16);
        assert_eq!(windows[1].1, 20);
    }

    #[test]
    fn test_extract_json_object_strips_surrounding_prose() {
        let raw = "Sure! Here's the JSON:\n```json\n{\"sections\":[]}\n```\nDone.";
        let extracted = extract_json_object(raw).unwrap();
        assert_eq!(extracted, "{\"sections\":[]}");
    }

    #[test]
    fn test_count_nodes_counts_recursively() {
        let tree = vec![PageIndexNode {
            id: "n1".into(),
            title: "A".into(),
            page_start: 1,
            page_end: 10,
            summary: String::new(),
            children: vec![PageIndexNode {
                id: "n2".into(),
                title: "A.1".into(),
                page_start: 1,
                page_end: 5,
                summary: String::new(),
                children: vec![],
            }],
        }];
        assert_eq!(count_nodes(&tree), 2);
    }
}
