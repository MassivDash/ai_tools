use actix_multipart::Multipart;
use actix_web::{post, web, HttpResponse, Result as ActixResult};
use crate::api::chromadb::client::ChromaDBClient;
use crate::api::chromadb::types::{AddDocumentsRequest, ChromaDBResponse};
use futures_util::TryStreamExt;
use uuid::Uuid;

#[post("/api/chromadb/documents/upload")]
pub async fn upload_documents(
    mut payload: Multipart,
    chroma_address: web::Data<String>,
) -> ActixResult<HttpResponse> {
    let client = match ChromaDBClient::new(chroma_address.as_str()) {
        Ok(c) => c,
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().json(ChromaDBResponse::<()> {
                success: false,
                data: None,
                error: Some(format!("Failed to initialize ChromaDB client: {}", e)),
                message: None,
            }));
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
        } else if filename.ends_with(".md") || filename.ends_with(".mdx") || filename.ends_with(".txt") {
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

        // Chunk the text
        let chunks = chunk_text(&text, 1000, 200); // chunk_size: 1000, overlap: 200
        
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
            println!("✅ Successfully added {} documents to collection {}", document_count, collection);
            Ok(HttpResponse::Ok().json(ChromaDBResponse {
                success: true,
                data: Some(()),
                error: None,
                message: Some(format!("Successfully uploaded {} documents to collection '{}'", document_count, collection)),
            }))
        }
        Err(e) => {
            println!("❌ Failed to add documents: {}", e);
            Ok(HttpResponse::InternalServerError().json(ChromaDBResponse::<()> {
                success: false,
                data: None,
                error: Some(format!("Failed to add documents to ChromaDB: {}", e)),
                message: None,
            }))
        }
    }
}

// PDF parser (placeholder - will need pdf-extract or similar crate)
fn parse_pdf(_data: &[u8]) -> Result<(String, std::collections::HashMap<String, String>), String> {
    // TODO: Implement PDF parsing using a crate like pdf-extract
    // For now, return an error
    Err("PDF parsing not yet implemented. Please use a crate like 'pdf-extract' or 'lopdf'".to_string())
}

// Text/Markdown parser
fn parse_text(data: &[u8]) -> Result<(String, std::collections::HashMap<String, String>), String> {
    let text = String::from_utf8(data.to_vec())
        .map_err(|e| format!("Failed to parse text: {}", e))?;
    
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("file_type".to_string(), "text".to_string());
    
    Ok((text, metadata))
}

// Text chunking strategy
fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut start = 0;

    while start < chars.len() {
        let end = std::cmp::min(start + chunk_size, chars.len());
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);
        
        if end >= chars.len() {
            break;
        }
        
        start = end.saturating_sub(overlap);
    }

    chunks
}

