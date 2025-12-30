use actix_multipart::Multipart;
use actix_web::{post, web, Error as ActixError, HttpResponse};
use arrow::array::{
    Array, BooleanArray, Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, Int8Array,
    LargeStringArray, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
};
use futures_util::{stream, TryStreamExt};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::time::{SystemTime, UNIX_EPOCH};

#[post("/api/parquet-to-txt")]
pub async fn convert_parquet_to_txt(mut payload: Multipart) -> Result<HttpResponse, ActixError> {
    let mut parquet_files: Vec<(String, Vec<u8>)> = Vec::new();

    // Parse multipart form data to collect all parquet files
    while let Some(mut field) = payload.try_next().await? {
        let field_name = field.name();

        if field_name == Some("files") {
            // Get filename from content disposition
            let content_disposition = field.content_disposition();
            let filename = content_disposition
                .as_ref()
                .and_then(|cd| cd.get_filename())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("file_{}.parquet", parquet_files.len()));

            // Read file data
            let mut data = Vec::new();
            while let Some(chunk) = field.try_next().await? {
                data.extend_from_slice(&chunk);
            }

            if !data.is_empty() {
                parquet_files.push((filename, data));
            }
        }
    }

    // Validate that we have at least one file
    if parquet_files.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No parquet files provided"
        })));
    }

    println!(
        "📥 Received {} parquet file(s) for conversion",
        parquet_files.len()
    );

    // Limit total file size to prevent memory issues (500MB max)
    const MAX_TOTAL_SIZE: usize = 500 * 1024 * 1024;
    let total_size: usize = parquet_files.iter().map(|(_, data)| data.len()).sum();
    if total_size > MAX_TOTAL_SIZE {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Total file size too large: {} bytes (max {} bytes)", total_size, MAX_TOTAL_SIZE)
        })));
    }

    // Create a stream that processes files and yields text chunks
    let total_files = parquet_files.len();

    // Generate filename for download
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let download_filename = format!("imatrix_quantization_data_{}.txt", timestamp);

    // Create a stream that processes files incrementally
    let stream = stream::unfold(
        (parquet_files, false),
        move |(mut files, started)| async move {
            if files.is_empty() && started {
                return None;
            }

            if !started {
                // Start processing
                return Some((
                    Ok(web::Bytes::from("")), // Empty first chunk to start
                    (files, true),
                ));
            }

            // Process next file (remove from front to maintain order)
            if !files.is_empty() {
                let (filename, file_data) = files.remove(0);
                println!(
                    "🔄 Processing parquet file: {} (size: {} bytes)",
                    filename,
                    file_data.len()
                );

                // Validate file is parquet
                if !filename.to_lowercase().ends_with(".parquet") {
                    println!("⚠️ Skipping non-parquet file: {}", filename);
                    return Some((Ok(web::Bytes::from("")), (files, true)));
                }

                match process_parquet_file(&file_data) {
                    Ok((text, rows)) => {
                        println!("✅ Processed {} rows from {}", rows, filename);
                        Some((Ok(web::Bytes::from(text)), (files, true)))
                    }
                    Err(e) => {
                        println!("Failed to process {}: {}", filename, e);
                        Some((
                            Err(ActixError::from(std::io::Error::other(format!(
                                "Failed to process {}: {}",
                                filename, e
                            )))),
                            (files, true),
                        ))
                    }
                }
            } else {
                None
            }
        },
    );

    println!(
        "✅ Streaming conversion of {} parquet file(s) to text",
        total_files
    );

    Ok(HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .append_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", download_filename),
        ))
        .streaming(stream))
}

/// Processes a single parquet file and extracts text data
fn process_parquet_file(data: &[u8]) -> Result<(String, usize), String> {
    // Convert Vec<u8> to Bytes which implements ChunkReader
    let bytes = web::Bytes::from(data.to_vec());

    // Build parquet reader
    let builder = ParquetRecordBatchReaderBuilder::try_new(bytes)
        .map_err(|e| format!("Failed to create parquet reader: {}", e))?;

    let schema = builder.schema().clone();
    let reader = builder
        .build()
        .map_err(|e| format!("Failed to build parquet reader: {}", e))?;

    let mut text_output = String::new();
    let mut total_rows = 0;

    // Read all record batches
    for batch_result in reader {
        let batch = batch_result.map_err(|e| format!("Failed to read record batch: {}", e))?;
        total_rows += batch.num_rows();

        // Process row by row to maintain data relationships
        for row_idx in 0..batch.num_rows() {
            let mut row_text = String::new();
            let mut has_data = false;

            // Extract values from each column for this row
            for (col_idx, _field) in schema.fields().iter().enumerate() {
                let column = batch.column(col_idx);

                if column.is_null(row_idx) {
                    continue;
                }

                let value_str = extract_value_from_array(column, row_idx);
                if !value_str.trim().is_empty() {
                    if has_data {
                        row_text.push(' ');
                    }
                    row_text.push_str(&value_str);
                    has_data = true;
                }
            }

            if has_data {
                text_output.push_str(&row_text);
                text_output.push('\n');
            }
        }
    }

    Ok((text_output, total_rows))
}

/// Extracts a string value from an arrow array at a specific row index
fn extract_value_from_array(array: &dyn Array, row_idx: usize) -> String {
    // Try different array types and extract the value
    if let Some(arr) = array.as_any().downcast_ref::<StringArray>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<LargeStringArray>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<Int8Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<Int16Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<Int32Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<Int64Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<UInt8Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<UInt16Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<UInt32Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<UInt64Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<Float32Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<Float64Array>() {
        arr.value(row_idx).to_string()
    } else if let Some(arr) = array.as_any().downcast_ref::<BooleanArray>() {
        arr.value(row_idx).to_string()
    } else {
        // For unsupported types, use debug representation
        format!("{:?}", array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_parquet_file_empty_data() {
        let empty_data = b"";
        let result = process_parquet_file(empty_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_parquet_file_invalid_data() {
        let invalid_data = b"This is not a parquet file";
        let result = process_parquet_file(invalid_data);
        assert!(result.is_err());
    }
}
