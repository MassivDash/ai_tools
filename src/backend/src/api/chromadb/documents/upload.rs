use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{AddDocumentsRequest, ChromaDBResponse};
use actix_multipart::Multipart;
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use futures_util::TryStreamExt;
use std::sync::Once;
use tokenizers::tokenizer::{Result as TokenizerResult, Tokenizer};
use uuid::Uuid;

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
            Box::new(io::Error::new(
                io::ErrorKind::Other,
                "Tokenizer initialization failed",
            )) as tokenizers::tokenizer::Error
        })
    }
}

#[post("/api/chromadb/documents/upload")]
pub async fn upload_documents(
    mut payload: Multipart,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            return Ok(
                HttpResponse::InternalServerError().json(ChromaDBResponse::<()> {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to initialize ChromaDB client: {}", e)),
                    message: None,
                }),
            );
        }
    };

    let mut collection_name: Option<String> = None;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    // Parse multipart form data
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let field_name = content_disposition.get_name().unwrap_or("");

        if field_name == "collection" {
            // Read collection name
            let mut bytes = Vec::new();
            while let Ok(Some(chunk)) = field.try_next().await {
                bytes.extend_from_slice(&chunk);
            }
            collection_name = String::from_utf8(bytes).ok();
        } else if field_name == "files" {
            // Read file data
            let filename = content_disposition
                .get_filename()
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

    // Validate inputs
    let collection = match collection_name {
        Some(name) if !name.is_empty() => name,
        _ => {
            return Ok(HttpResponse::BadRequest().json(ChromaDBResponse::<()> {
                success: false,
                data: None,
                error: Some("Collection name is required".to_string()),
                message: None,
            }));
        }
    };

    if files.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ChromaDBResponse::<()> {
            success: false,
            data: None,
            error: Some("At least one file is required".to_string()),
            message: None,
        }));
    }

    // Process files
    let mut all_documents: Vec<String> = Vec::new();
    let mut all_ids: Vec<String> = Vec::new();
    let mut all_metadatas: Vec<std::collections::HashMap<String, String>> = Vec::new();

    for (filename, file_data) in files {
        println!("📄 Processing file: {}", filename);

        // Determine file type and parse
        let (text, metadata) = if filename.ends_with(".pdf") {
            match parse_pdf(&file_data) {
                Ok((text, meta)) => (text, meta),
                Err(e) => {
                    println!("❌ Error parsing PDF {}: {}", filename, e);
                    continue;
                }
            }
        } else if filename.ends_with(".md")
            || filename.ends_with(".mdx")
            || filename.ends_with(".txt")
        {
            match parse_text(&file_data) {
                Ok((text, meta)) => (text, meta),
                Err(e) => {
                    println!("❌ Error parsing text file {}: {}", filename, e);
                    continue;
                }
            }
        } else {
            println!("⚠️ Unsupported file type: {}", filename);
            continue;
        };

        // Chunk the text using token-based semantic chunking
        // For markdown files, use markdown-aware chunking; otherwise use semantic chunking
        // Optimal for nomic-embed-text: 512 tokens per chunk, 50 token overlap
        let chunks = match get_tokenizer() {
            Ok(tokenizer) => {
                if filename.ends_with(".md") || filename.ends_with(".mdx") {
                    chunk_markdown_semantic_tokens(&text, tokenizer, 512, 50)
                } else {
                    chunk_semantic_tokens(&text, tokenizer, 512, 50)
                }
            }
            Err(e) => {
                println!(
                    "⚠️ Tokenizer error: {:?}. Falling back to character-based chunking.",
                    e
                );
                // Fallback to character-based chunking
                if filename.ends_with(".md") || filename.ends_with(".mdx") {
                    chunk_markdown_semantic(&text, 1500, 200)
                } else {
                    chunk_semantic(&text, 1500, 200)
                }
            }
        };

        // Create documents, IDs, and metadata for each chunk
        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let chunk_id = Uuid::new_v4().to_string();
            all_ids.push(chunk_id);
            all_documents.push(chunk.clone());

            // Create metadata for this chunk
            let mut chunk_metadata = metadata.clone();
            chunk_metadata.insert("filename".to_string(), filename.clone());
            chunk_metadata.insert("chunk_index".to_string(), chunk_idx.to_string());
            chunk_metadata.insert("total_chunks".to_string(), chunks.len().to_string());
            all_metadatas.push(chunk_metadata);
        }
    }

    if all_documents.is_empty() {
        return Ok(HttpResponse::BadRequest().json(ChromaDBResponse::<()> {
            success: false,
            data: None,
            error: Some("No valid documents were extracted from the files".to_string()),
            message: None,
        }));
    }

    // Add documents to ChromaDB
    let document_count = all_documents.len();
    let request = AddDocumentsRequest {
        collection: collection.clone(),
        ids: all_ids,
        documents: all_documents,
        metadatas: Some(all_metadatas),
    };

    match client.add_documents(request).await {
        Ok(_) => {
            println!(
                "✅ Successfully added {} documents to collection {}",
                document_count, collection
            );
            Ok(HttpResponse::Ok().json(ChromaDBResponse {
                success: true,
                data: Some(()),
                error: None,
                message: Some(format!(
                    "Successfully uploaded {} documents to collection '{}'",
                    document_count, collection
                )),
            }))
        }
        Err(e) => {
            println!("❌ Failed to add documents: {}", e);
            Ok(
                HttpResponse::InternalServerError().json(ChromaDBResponse::<()> {
                    success: false,
                    data: None,
                    error: Some(format!("Failed to add documents to ChromaDB: {}", e)),
                    message: None,
                }),
            )
        }
    }
}

// PDF parser (placeholder - will need pdf-extract or similar crate)
fn parse_pdf(_data: &[u8]) -> Result<(String, std::collections::HashMap<String, String>), String> {
    // TODO: Implement PDF parsing using a crate like pdf-extract
    // For now, return an error
    Err(
        "PDF parsing not yet implemented. Please use a crate like 'pdf-extract' or 'lopdf'"
            .to_string(),
    )
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

            if current_token_count < target_tokens / 2 {
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

                    let test_sentence_chunk = if current_chunk.is_empty() {
                        sentence.to_string()
                    } else {
                        format!("{}. {}", current_chunk, sentence)
                    };

                    let sentence_token_count =
                        match tokenizer.encode(test_sentence_chunk.as_str(), false) {
                            Ok(encoding) => encoding.len(),
                            Err(_) => test_sentence_chunk.len() / 4,
                        };

                    if sentence_token_count > target_tokens {
                        if !current_chunk.is_empty() {
                            chunks.push(current_chunk.trim().to_string());
                            // Start new chunk with overlap from previous chunk
                            current_chunk = get_overlap_text(&chunks, tokenizer, overlap_tokens);
                        }
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
                current_chunk = get_overlap_text(&chunks, tokenizer, overlap_tokens);
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
            current_section.push_str(line);
            current_section.push('\n');
        } else {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    // Add final section
    if !current_section.trim().is_empty() {
        sections.push(current_section);
    }

    // If we have multiple sections, chunk each section semantically
    if sections.len() > 1 {
        let mut all_chunks = Vec::new();
        for section in sections {
            let section_chunks =
                chunk_semantic_tokens(&section, tokenizer, target_tokens, overlap_tokens);
            all_chunks.extend(section_chunks);
        }
        if !all_chunks.is_empty() {
            return all_chunks;
        }
    }

    // Fallback to regular semantic token-based chunking
    chunk_semantic_tokens(text, tokenizer, target_tokens, overlap_tokens)
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

                    if current_chunk.len() + sentence.len() + 2 > target_chunk_size {
                        if !current_chunk.is_empty() {
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
            current_section.push_str(line);
            current_section.push('\n');
        } else {
            current_section.push_str(line);
            current_section.push('\n');
        }
    }

    // Add final section
    if !current_section.trim().is_empty() {
        sections.push(current_section);
    }

    // If we have multiple sections, chunk each section semantically
    if sections.len() > 1 {
        let mut all_chunks = Vec::new();
        for section in sections {
            let section_chunks = chunk_semantic(&section, target_chunk_size, overlap);
            all_chunks.extend(section_chunks);
        }
        if !all_chunks.is_empty() {
            return all_chunks;
        }
    }

    // Fallback to regular semantic chunking if header-based splitting didn't help
    chunk_semantic(text, target_chunk_size, overlap)
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
