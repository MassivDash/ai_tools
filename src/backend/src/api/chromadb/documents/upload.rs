use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::config::types::ChromaDBConfig;
use crate::api::chromadb::types::AddDocumentsRequest;
use actix_multipart::Multipart;
use actix_web::{post, web, HttpResponse};
use bytes::Bytes;
use futures_util::TryStreamExt;
use std::sync::Once;
use tokenizers::tokenizer::{Result as TokenizerResult, Tokenizer};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

// Global tokenizer instance - initialized once and reused
// Using GPT-2 style BPE tokenizer which is compatible with nomic-embed-text
static TOKENIZER_INIT: Once = Once::new();
static mut TOKENIZER: Option<Tokenizer> = None;

fn get_tokenizer() -> TokenizerResult<&'static Tokenizer> {
    unsafe {
        let mut init_error: Option<tokenizers::tokenizer::Error> = None;
        TOKENIZER_INIT.call_once(|| {
            // Try to load GPT-2 tokenizer (compatible with nomic-embed-text style BPE)
            match Tokenizer::from_pretrained("gpt2", None) {
                Ok(tok) => {
                    println!("✅ Loaded GPT-2 tokenizer for token-based chunking");
                    TOKENIZER = Some(tok);
                }
                Err(e) => {
                    println!(
                        "⚠️ Failed to load GPT-2 tokenizer: {:?}. Will retry on next call.",
                        e
                    );
                    init_error = Some(e);
                }
            }
        });

        if let Some(err) = init_error {
            return Err(err);
        }

        // SAFETY: TOKENIZER is only written to during initialization (call_once),
        // and after that it's only read. This is safe because Once ensures single initialization.
        #[allow(static_mut_refs)]
        TOKENIZER.as_ref().ok_or_else(|| {
            // Create an error from a string message
            use std::io;
            Box::new(io::Error::other("Tokenizer initialization failed"))
                as tokenizers::tokenizer::Error
        })
    }
}

#[post("/api/chromadb/documents/upload")]
pub async fn upload_documents(
    mut payload: Multipart,
    chroma_address: web::Data<String>,
    chromadb_config: web::Data<std::sync::Arc<std::sync::Mutex<ChromaDBConfig>>>,
) -> Result<HttpResponse, actix_web::Error> {
    let mut collection_name: Option<String> = None;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    // Parse multipart form data completely before spawning thread (Multipart is !Send)
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition
            .as_ref()
            .and_then(|cd| cd.get_name())
            .unwrap_or("");

        if field_name == "collection" {
            let mut bytes = Vec::new();
            while let Ok(Some(chunk)) = field.try_next().await {
                bytes.extend_from_slice(&chunk);
            }
            collection_name = String::from_utf8(bytes).ok();
        } else if field_name == "files" {
            let filename = content_disposition
                .as_ref()
                .and_then(|cd| cd.get_filename())
                .unwrap_or("unknown")
                .to_string();

            let mut file_data = Vec::new();
            while let Ok(Some(chunk)) = field.try_next().await {
                file_data.extend_from_slice(&chunk);
            }

            if !file_data.is_empty() {
                files.push((filename, file_data));
            }
        }
    }

    let (tx, rx) = mpsc::channel::<Bytes>(100);
    let tx_clone = tx.clone();

    // Helper to send SSE messages
    let send_sse = |status: &str,
                    message: &str,
                    success: Option<bool>,
                    processed: Option<usize>,
                    total: Option<usize>| {
        let mut val = serde_json::json!({
            "status": status,
            "message": message
        });
        if let Some(s) = success {
            val["success"] = serde_json::json!(s);
        }
        if let Some(p) = processed {
            val["processed_files"] = serde_json::json!(p);
        }
        if let Some(t) = total {
            val["total_files"] = serde_json::json!(t);
        }
        let sse_str = format!("data: {}\n\n", val);
        Bytes::from(sse_str)
    };

    // Validate inputs
    let collection = match collection_name {
        Some(name) if !name.is_empty() => name,
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Collection name is required"
            })));
        }
    };

    if files.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "At least one file is required"
        })));
    }

    // Get config for chunking
    let (chunk_size, chunk_overlap, default_embedding_model) = {
        let config_guard = chromadb_config.lock().unwrap();
        (
            config_guard.chunk_size,
            config_guard.chunk_overlap,
            config_guard.embedding_model.clone(),
        )
    };

    let chroma_addr = chroma_address.to_string();

    tokio::spawn(async move {
        // Initialize ChromaDB client manually
        let client = match ChromaDBClient::new(&chroma_addr) {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to connect to ChromaDB: {}", e);
                let _ = tx_clone
                    .send(send_sse("error", &err_msg, Some(false), None, None))
                    .await;
                return;
            }
        };

        // Fetch the collection to see if it has an embedding_model attached to its metadata
        let mut embedding_model = default_embedding_model;
        if let Ok(collection_info) = client.get_collection(&collection).await {
            if let Some(metadata) = collection_info.metadata {
                if let Some(model) = metadata.get("embedding_model") {
                    embedding_model = model.to_string();
                }
            }
        }

        // Send initial status immediately so connection opens

        let _ = tx_clone
            .send(send_sse("info", "Starting processing...", None, None, None))
            .await;

        let msg = format!(
            "⚙️ Using chunk size: {}, overlap: {}",
            chunk_size, chunk_overlap
        );
        println!("{}", msg);
        let _ = tx_clone
            .send(send_sse("info", &msg, None, None, None))
            .await;

        let mut successful_files = 0;
        let mut failed_files = Vec::new();
        let total_files = files.len();
        let mut all_documents: Vec<String> = Vec::new();
        let mut all_ids: Vec<String> = Vec::new();
        let mut all_metadatas: Vec<std::collections::HashMap<String, String>> = Vec::new();

        let docs_dir = std::path::Path::new("./public/documents");
        if let Err(e) = std::fs::create_dir_all(docs_dir) {
            println!("⚠️ Failed to create documents directory: {}", e);
        }

        for (filename, file_data) in files {
            let msg = format!("📄 Processing file: {}", filename);
            println!("{}", msg);
            let _ = tx_clone
                .send(send_sse(
                    "processing",
                    &msg,
                    None,
                    Some(successful_files),
                    Some(total_files),
                ))
                .await;

            let file_path = docs_dir.join(&filename);
            if let Err(e) = std::fs::write(&file_path, &file_data) {
                println!("⚠️ Failed to save document to {:?}: {}", file_path, e);
            }

            let (text, metadata, final_filename) = if filename.ends_with(".pdf") {
                match parse_pdf(&file_data) {
                    Ok((raw_text, meta)) => {
                        let md_text = format_text_as_markdown(&raw_text);
                        let md_filename = format!("{}.md", filename);
                        let md_path = docs_dir.join(&md_filename);
                        if let Err(e) = std::fs::write(&md_path, &md_text) {
                            println!("⚠️ Failed to save markdown to {:?}: {}", md_path, e);
                        }
                        (md_text, meta, md_filename)
                    }
                    Err(e) => {
                        let err_msg = format!("Error parsing PDF {}: {}", filename, e);
                        println!("{}", err_msg);
                        let _ = tx_clone
                            .send(send_sse("error", &err_msg, None, None, None))
                            .await;
                        failed_files.push(filename.clone());
                        continue;
                    }
                }
            } else if filename.ends_with(".md")
                || filename.ends_with(".mdx")
                || filename.ends_with(".txt")
            {
                match parse_text(&file_data) {
                    Ok((text, meta)) => (text, meta, filename.clone()),
                    Err(e) => {
                        let err_msg = format!("Error parsing text file {}: {}", filename, e);
                        println!("{}", err_msg);
                        let _ = tx_clone
                            .send(send_sse("error", &err_msg, None, None, None))
                            .await;
                        failed_files.push(filename.clone());
                        continue;
                    }
                }
            } else {
                let err_msg = format!("⚠️ Unsupported file type: {}", filename);
                println!("{}", err_msg);
                let _ = tx_clone
                    .send(send_sse("error", &err_msg, None, None, None))
                    .await;
                failed_files.push(filename.clone());
                continue;
            };

            successful_files += 1;

            let chunks = match get_tokenizer() {
                Ok(tokenizer) => {
                    let _ = tx_clone
                        .send(send_sse(
                            "info",
                            "✅ Loaded GPT-2 tokenizer for token-based chunking",
                            None,
                            None,
                            None,
                        ))
                        .await;
                    if final_filename.ends_with(".md") || final_filename.ends_with(".mdx") {
                        chunk_markdown_semantic_tokens(&text, tokenizer, chunk_size, chunk_overlap)
                    } else {
                        chunk_semantic_tokens(&text, tokenizer, chunk_size, chunk_overlap)
                    }
                }
                Err(e) => {
                    let err_msg = format!(
                        "⚠️ Tokenizer error: {:?}. Falling back to character-based chunking.",
                        e
                    );
                    println!("{}", err_msg);
                    let _ = tx_clone
                        .send(send_sse("warning", &err_msg, None, None, None))
                        .await;
                    let char_chunk_size = chunk_size * 3;
                    let char_overlap = chunk_overlap * 3;
                    if final_filename.ends_with(".md") || final_filename.ends_with(".mdx") {
                        chunk_markdown_semantic(&text, char_chunk_size, char_overlap)
                    } else {
                        chunk_semantic(&text, char_chunk_size, char_overlap)
                    }
                }
            };

            for (chunk_idx, chunk) in chunks.iter().enumerate() {
                if chunk.trim().is_empty() {
                    continue;
                }
                let chunk_id = uuid::Uuid::new_v4().to_string();
                all_ids.push(chunk_id);
                all_documents.push(chunk.clone());

                let mut chunk_metadata = metadata.clone();
                chunk_metadata.insert("filename".to_string(), final_filename.clone());
                chunk_metadata.insert("chunk_index".to_string(), chunk_idx.to_string());
                chunk_metadata.insert("total_chunks".to_string(), chunks.len().to_string());
                all_metadatas.push(chunk_metadata);
            }
        }

        if all_documents.is_empty() {
            let _ = tx_clone
                .send(send_sse(
                    "error",
                    "No valid documents were extracted from the files",
                    Some(false),
                    None,
                    None,
                ))
                .await;
            return;
        }

        let document_count = all_documents.len();
        let request = AddDocumentsRequest {
            collection: collection.clone(),
            ids: all_ids,
            documents: all_documents,
            metadatas: Some(all_metadatas),
        };

        let msg = format!("Pushing {} chunks to ChromaDB...", document_count);
        let _ = tx_clone
            .send(send_sse("info", &msg, None, None, None))
            .await;

        let tx_ping = tx_clone.clone();
        let ping_task = tokio::spawn(async move {
            let mut seconds = 0;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                seconds += 5;
                let ping_msg = format!(
                    "Generating embeddings for {} chunks... Elapsed: {}s",
                    document_count, seconds
                );
                let _ = tx_ping
                    .send(send_sse("processing", &ping_msg, None, None, None))
                    .await;
            }
        });

        let add_result = client.add_documents(request, &embedding_model).await;
        ping_task.abort();

        match add_result {
            Ok(_) => {
                let mut result_msg = format!(
                    "Successfully processed {} file(s) into {} chunks for collection '{}'",
                    successful_files, document_count, collection
                );

                if !failed_files.is_empty() {
                    result_msg
                        .push_str(&format!(". Failed to process: {}", failed_files.join(", ")));
                }

                let _ = tx_clone
                    .send(send_sse("completed", &result_msg, Some(true), None, None))
                    .await;
            }
            Err(e) => {
                let err_msg = format!("Failed to add documents to ChromaDB: {}", e);
                println!("{}", err_msg);
                let _ = tx_clone
                    .send(send_sse("error", &err_msg, Some(false), None, None))
                    .await;
            }
        }
    });

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .append_header(("Cache-Control", "no-cache"))
        .append_header(("Connection", "keep-alive"))
        .streaming(futures_util::StreamExt::map(
            ReceiverStream::new(rx),
            Ok::<_, actix_web::Error>,
        )))
}

// PDF parser using pdftotext (external tool)
fn parse_pdf(data: &[u8]) -> Result<(String, std::collections::HashMap<String, String>), String> {
    use std::io::Write;
    use std::process::Command;
    use tempfile::NamedTempFile;

    // Create a temporary file to write the PDF data to
    let mut temp_file =
        NamedTempFile::new().map_err(|e| format!("Failed to create temp file: {}", e))?;
    temp_file
        .write_all(data)
        .map_err(|e| format!("Failed to write PDF data: {}", e))?;

    let temp_path = temp_file.path().to_owned();

    // Call pdftotext
    let output = Command::new("pdftotext")
        .arg("-layout") // Maintain layout
        .arg("-enc")
        .arg("UTF-8")
        .arg(&temp_path)
        .arg("-") // Output to stdout
        .output()
        .map_err(|e| format!("Failed to execute pdftotext: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftotext failed: {}", stderr));
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|e| format!("Invalid UTF-8 output from pdftotext: {}", e))?;

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("file_type".to_string(), "pdf".to_string());
    metadata.insert("parser".to_string(), "pdftotext".to_string());

    Ok((text, metadata))
}

// Text/Markdown parser
fn parse_text(data: &[u8]) -> Result<(String, std::collections::HashMap<String, String>), String> {
    let text =
        String::from_utf8(data.to_vec()).map_err(|e| format!("Failed to parse text: {}", e))?;

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("file_type".to_string(), "text".to_string());

    Ok((text, metadata))
}

// Token-based semantic chunking - uses tokenizer to count tokens accurately
// This is the industry standard approach for embedding models
fn chunk_semantic_tokens(
    text: &str,
    tokenizer: &Tokenizer,
    target_tokens: usize,
    overlap_tokens: usize,
) -> Vec<String> {
    let mut chunks = Vec::new();

    // First, split by double newlines (paragraphs)
    let paragraphs: Vec<&str> = text.split("\n\n").collect();

    let mut current_chunk = String::new();

    for paragraph in paragraphs {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }

        // Check token count of current chunk + new paragraph
        let test_chunk = if current_chunk.is_empty() {
            paragraph.to_string()
        } else {
            format!("{}\n\n{}", current_chunk, paragraph)
        };

        let token_count = match tokenizer.encode(test_chunk.as_str(), false) {
            Ok(encoding) => encoding.len(),
            Err(_) => {
                // Fallback to character-based estimation if tokenization fails
                test_chunk.len() / 4 // Rough estimate: ~4 chars per token
            }
        };

        // If adding this paragraph would exceed target tokens, finalize current chunk
        if !current_chunk.is_empty() && token_count > target_tokens {
            // Try to split at sentence boundaries within the paragraph if needed
            let current_token_count = match tokenizer.encode(current_chunk.as_str(), false) {
                Ok(encoding) => encoding.len(),
                Err(_) => current_chunk.len() / 4,
            };

            // If current chunk is already big enough (>= 90% of target), push it and start new
            // This prevents trying to squeeze too much in and accidentally going over
            if current_token_count >= (target_tokens * 9 / 10) {
                chunks.push(current_chunk.trim().to_string());
                // Start new chunk with overlap
                current_chunk = get_overlap_text(&chunks, tokenizer, overlap_tokens);
            }

            // Now handle the paragraph sentence by sentence
            let sentences: Vec<&str> = paragraph
                .split(&['.', '!', '?', '\n'][..])
                .filter(|s| !s.trim().is_empty())
                .collect();

            // Now iterate sentences and add them, splitting if necessary
            for sentence in sentences {
                let sentence = sentence.trim();
                let sentence_token_count = match tokenizer.encode(sentence, false) {
                    Ok(e) => e.len(),
                    Err(_) => sentence.len() / 4,
                };

                // If sentence itself is bigger than target, we MUST hard split it
                if sentence_token_count > target_tokens {
                    // Push current chunk if not empty
                    if !current_chunk.is_empty() {
                        chunks.push(current_chunk.trim().to_string());
                        // No need to calculate overlap as we are breaking context here
                        current_chunk = String::new();
                    }

                    // Split the huge sentence into smaller chunks
                    // We'll use character approximation to be safe since token splitting is hard
                    // Assumes 1 token ~= 3 chars to be safe (conservative)
                    let safe_char_limit = target_tokens * 3;
                    let mut sentence_remaining = sentence;

                    while !sentence_remaining.is_empty() {
                        if sentence_remaining.len() <= safe_char_limit {
                            chunks.push(sentence_remaining.to_string());
                            break;
                        }

                        // Find a split point
                        let split_idx = if let Some((idx, _)) =
                            sentence_remaining.char_indices().nth(safe_char_limit)
                        {
                            idx
                        } else {
                            sentence_remaining.len()
                        };

                        chunks.push(sentence_remaining[..split_idx].to_string());
                        sentence_remaining = &sentence_remaining[split_idx..];
                    }
                    continue;
                }

                // Check if adding sentence exceeds limit
                let next_chunk_tokens = match tokenizer
                    .encode(format!("{}. {}", current_chunk, sentence).as_str(), false)
                {
                    Ok(e) => e.len(),
                    Err(_) => (current_chunk.len() + sentence.len()) / 4,
                };

                if next_chunk_tokens > target_tokens {
                    // Push current and start new
                    if !current_chunk.is_empty() {
                        chunks.push(current_chunk.trim().to_string());
                        current_chunk = get_overlap_text(&chunks, tokenizer, overlap_tokens);
                    }
                }

                if !current_chunk.is_empty() {
                    current_chunk.push_str(". ");
                }
                current_chunk.push_str(sentence);
            }
        } else {
            // Add paragraph to current chunk
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(paragraph);
        }
    }

    // Add final chunk if not empty
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    // Fallback: if no chunks were created, use character-based
    if chunks.is_empty() {
        return chunk_text_fallback(text, target_tokens * 4, overlap_tokens * 4);
    }

    chunks
}

// Helper function to get overlap text from previous chunk using tokenizer
fn get_overlap_text(chunks: &[String], tokenizer: &Tokenizer, overlap_tokens: usize) -> String {
    if chunks.is_empty() || overlap_tokens == 0 {
        return String::new();
    }

    let last_chunk = &chunks[chunks.len() - 1];

    // Encode the last chunk to get tokens
    match tokenizer.encode(last_chunk.as_str(), false) {
        Ok(encoding) => {
            let tokens = encoding.get_ids();
            if tokens.len() <= overlap_tokens {
                return last_chunk.clone();
            }

            // Take the last overlap_tokens tokens
            let overlap_token_ids: Vec<u32> = tokens[tokens.len() - overlap_tokens..].to_vec();

            // Decode back to text
            match tokenizer.decode(&overlap_token_ids, true) {
                Ok(text) => text,
                Err(_) => {
                    // Fallback: use character-based overlap
                    let char_count = last_chunk.chars().count();
                    let chars_to_keep = (overlap_tokens * 4).min(char_count);
                    let overlap_start = last_chunk
                        .char_indices()
                        .nth(char_count.saturating_sub(chars_to_keep))
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);
                    last_chunk[overlap_start..].to_string()
                }
            }
        }
        Err(_) => {
            // Fallback: use character-based overlap
            let char_count = last_chunk.chars().count();
            let chars_to_keep = (overlap_tokens * 4).min(char_count);
            let overlap_start = last_chunk
                .char_indices()
                .nth(char_count.saturating_sub(chars_to_keep))
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            last_chunk[overlap_start..].to_string()
        }
    }
}

// Markdown-aware token-based semantic chunking
fn chunk_markdown_semantic_tokens(
    text: &str,
    tokenizer: &Tokenizer,
    target_tokens: usize,
    overlap_tokens: usize,
) -> Vec<String> {
    // First, try to split by major markdown headers (## or ###)
    let lines: Vec<&str> = text.lines().collect();
    let mut sections = Vec::new();
    let mut current_section = String::new();

    for line in lines {
        let trimmed = line.trim();
        // Check if this is a markdown header (starts with ## or ###)
        if trimmed.starts_with("##") {
            // Save previous section if not empty
            if !current_section.trim().is_empty() {
                sections.push(current_section);
                current_section = String::new();
            }
        }
        current_section.push_str(line);
        current_section.push('\n');
    }

    // Add final section
    if !current_section.trim().is_empty() {
        sections.push(current_section);
    }

    let mut all_chunks = Vec::new();
    let mut current_group = String::new();

    for section in sections {
        let test_combined = if current_group.is_empty() {
            section.clone()
        } else {
            format!("{}{}", current_group, section)
        };

        let token_count = match tokenizer.encode(test_combined.as_str(), false) {
            Ok(e) => e.len(),
            Err(_) => test_combined.len() / 4,
        };

        // If combining exceeds target and we already have content
        if token_count > target_tokens && !current_group.is_empty() {
            // Process the current_group
            let group_chunks =
                chunk_semantic_tokens(&current_group, tokenizer, target_tokens, overlap_tokens);
            all_chunks.extend(group_chunks);

            // Get overlap from the end of what we just processed
            let overlap_str = get_overlap_text(&all_chunks, tokenizer, overlap_tokens);

            // Start a new group with overlap + section
            current_group = format!("{}{}", overlap_str, section);
        } else {
            current_group = test_combined;
        }
    }

    // Process any remaining grouped sections
    if !current_group.trim().is_empty() {
        let group_chunks =
            chunk_semantic_tokens(&current_group, tokenizer, target_tokens, overlap_tokens);
        all_chunks.extend(group_chunks);
    }

    if all_chunks.is_empty() {
        return chunk_semantic_tokens(text, tokenizer, target_tokens, overlap_tokens);
    }

    all_chunks
}

// Semantic chunking strategy - respects sentence and paragraph boundaries (character-based fallback)
// This is the recommended approach for vector databases as it preserves semantic meaning
fn chunk_semantic(text: &str, target_chunk_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();

    // First, split by double newlines (paragraphs)
    let paragraphs: Vec<&str> = text.split("\n\n").collect();

    let mut current_chunk = String::new();

    for paragraph in paragraphs {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }

        // If adding this paragraph would exceed target size, finalize current chunk
        if !current_chunk.is_empty()
            && current_chunk.len() + paragraph.len() + 2 > target_chunk_size
        {
            // Try to split at sentence boundaries within the paragraph if needed
            if current_chunk.len() < target_chunk_size / 2 {
                // Current chunk is too small, try to add part of the paragraph
                let sentences: Vec<&str> = paragraph
                    .split(&['.', '!', '?', '\n'][..])
                    .filter(|s| !s.trim().is_empty())
                    .collect();

                for sentence in sentences {
                    let sentence = sentence.trim();
                    if sentence.is_empty() {
                        continue;
                    }

                    if current_chunk.len() + sentence.len() + 2 > target_chunk_size
                        && !current_chunk.is_empty()
                    {
                        chunks.push(current_chunk.trim().to_string());
                        // Start new chunk with overlap from previous chunk
                        current_chunk = if overlap > 0 && !chunks.is_empty() {
                            let last_chunk = &chunks[chunks.len() - 1];
                            // Find safe UTF-8 boundary for overlap (go back 'overlap' characters)
                            let char_count = last_chunk.chars().count();
                            let chars_to_keep = overlap.min(char_count);
                            let overlap_start = last_chunk
                                .char_indices()
                                .nth(char_count.saturating_sub(chars_to_keep))
                                .map(|(idx, _)| idx)
                                .unwrap_or(0);
                            last_chunk[overlap_start..].to_string()
                        } else {
                            String::new()
                        };
                    }

                    if !current_chunk.is_empty() {
                        current_chunk.push_str(". ");
                    }
                    current_chunk.push_str(sentence);
                }
            } else {
                // Current chunk is substantial, save it and start new one
                chunks.push(current_chunk.trim().to_string());
                // Start new chunk with overlap from previous chunk
                current_chunk = if overlap > 0 && !chunks.is_empty() {
                    let last_chunk = &chunks[chunks.len() - 1];
                    // Find safe UTF-8 boundary for overlap (go back 'overlap' characters)
                    let char_count = last_chunk.chars().count();
                    let chars_to_keep = overlap.min(char_count);
                    let overlap_start = last_chunk
                        .char_indices()
                        .nth(char_count.saturating_sub(chars_to_keep))
                        .map(|(idx, _)| idx)
                        .unwrap_or(0);
                    last_chunk[overlap_start..].to_string()
                } else {
                    String::new()
                };
                current_chunk.push_str(paragraph);
            }
        } else {
            // Add paragraph to current chunk
            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(paragraph);
        }
    }

    // Add final chunk if not empty
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    // Fallback: if no chunks were created (e.g., single long line), use character-based
    if chunks.is_empty() {
        return chunk_text_fallback(text, target_chunk_size, overlap);
    }

    chunks
}

// Markdown-aware semantic chunking - respects markdown structure (headers, lists, code blocks)
fn chunk_markdown_semantic(text: &str, target_chunk_size: usize, overlap: usize) -> Vec<String> {
    // First, try to split by major markdown headers (## or ###)
    // This preserves document structure better than pure paragraph splitting
    let lines: Vec<&str> = text.lines().collect();
    let mut sections = Vec::new();
    let mut current_section = String::new();

    for line in lines {
        let trimmed = line.trim();
        // Check if this is a markdown header (starts with ## or ###)
        if trimmed.starts_with("##") {
            // Save previous section if not empty
            if !current_section.trim().is_empty() {
                sections.push(current_section);
                current_section = String::new();
            }
        }
        current_section.push_str(line);
        current_section.push('\n');
    }

    // Add final section
    if !current_section.trim().is_empty() {
        sections.push(current_section);
    }

    let mut all_chunks = Vec::new();
    let mut current_group = String::new();

    for section in sections {
        let test_combined = if current_group.is_empty() {
            section.clone()
        } else {
            format!("{}{}", current_group, section)
        };

        if test_combined.len() > target_chunk_size && !current_group.is_empty() {
            let group_chunks = chunk_semantic(&current_group, target_chunk_size, overlap);
            all_chunks.extend(group_chunks);

            // Start new group with overlap
            let overlap_str = if overlap > 0 && !all_chunks.is_empty() {
                let last_chunk = &all_chunks[all_chunks.len() - 1];
                let char_count = last_chunk.chars().count();
                let chars_to_keep = overlap.min(char_count);
                let overlap_start = last_chunk
                    .char_indices()
                    .nth(char_count.saturating_sub(chars_to_keep))
                    .map(|(idx, _)| idx)
                    .unwrap_or(0);
                last_chunk[overlap_start..].to_string()
            } else {
                String::new()
            };

            current_group = format!("{}{}", overlap_str, section);
        } else {
            current_group = test_combined;
        }
    }

    if !current_group.trim().is_empty() {
        let group_chunks = chunk_semantic(&current_group, target_chunk_size, overlap);
        all_chunks.extend(group_chunks);
    }

    if all_chunks.is_empty() {
        return chunk_semantic(text, target_chunk_size, overlap);
    }

    all_chunks
}

// Fallback: Simple character-based chunking (original implementation)
// Used when semantic chunking fails or for very uniform text
fn chunk_text_fallback(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;

    while start < chars.len() {
        let end = std::cmp::min(start + chunk_size, chars.len());
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk.trim().to_string());

        if end >= chars.len() {
            break;
        }

        start = end.saturating_sub(overlap);
    }

    chunks
}

/// Formats plain text as markdown
fn format_text_as_markdown(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut markdown = String::new();
    let mut prev_empty = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !prev_empty {
                markdown.push_str("\n\n");
                prev_empty = true;
            }
        } else {
            // Preserve the line, but ensure proper spacing
            markdown.push_str(trimmed);
            markdown.push('\n');
            prev_empty = false;
        }
    }

    markdown.trim().to_string()
}
