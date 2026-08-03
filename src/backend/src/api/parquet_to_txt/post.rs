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
    use super::test_support::{sample_parquet, to_parquet};
    use super::*;
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

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

    #[test]
    fn test_process_parquet_file_joins_columns_and_skips_nulls() {
        let (text, rows) = process_parquet_file(&sample_parquet()).unwrap();

        assert_eq!(rows, 3);
        // Null cells are dropped; the remaining values in a row are space-joined.
        assert_eq!(text, "hello world 1\nsecond row\n3\n");
    }

    #[test]
    fn test_process_parquet_file_with_zero_rows() {
        let schema = Arc::new(Schema::new(vec![Field::new("text", DataType::Utf8, true)]));
        let batch = RecordBatch::try_new(
            schema,
            vec![Arc::new(StringArray::from(Vec::<Option<&str>>::new()))],
        )
        .unwrap();

        let (text, rows) = process_parquet_file(&to_parquet(&batch)).unwrap();

        assert_eq!(rows, 0);
        assert!(text.is_empty());
    }

    #[test]
    fn test_process_parquet_file_drops_rows_that_are_entirely_blank() {
        let schema = Arc::new(Schema::new(vec![Field::new("text", DataType::Utf8, true)]));
        let batch = RecordBatch::try_new(
            schema,
            vec![Arc::new(StringArray::from(vec![
                Some("kept"),
                None,
                Some("   "),
                Some("also kept"),
            ]))],
        )
        .unwrap();

        let (text, rows) = process_parquet_file(&to_parquet(&batch)).unwrap();

        // All four rows are counted, but nulls and whitespace-only cells emit nothing.
        assert_eq!(rows, 4);
        assert_eq!(text, "kept\nalso kept\n");
    }

    #[test]
    fn test_process_parquet_file_handles_every_supported_column_type() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("s", DataType::Utf8, false),
            Field::new("ls", DataType::LargeUtf8, false),
            Field::new("i8", DataType::Int8, false),
            Field::new("i16", DataType::Int16, false),
            Field::new("i32", DataType::Int32, false),
            Field::new("i64", DataType::Int64, false),
            Field::new("u8", DataType::UInt8, false),
            Field::new("u16", DataType::UInt16, false),
            Field::new("u32", DataType::UInt32, false),
            Field::new("u64", DataType::UInt64, false),
            Field::new("f32", DataType::Float32, false),
            Field::new("f64", DataType::Float64, false),
            Field::new("b", DataType::Boolean, false),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["str"])),
                Arc::new(LargeStringArray::from(vec!["large"])),
                Arc::new(Int8Array::from(vec![-8i8])),
                Arc::new(Int16Array::from(vec![-16i16])),
                Arc::new(Int32Array::from(vec![-32i32])),
                Arc::new(Int64Array::from(vec![-64i64])),
                Arc::new(UInt8Array::from(vec![8u8])),
                Arc::new(UInt16Array::from(vec![16u16])),
                Arc::new(UInt32Array::from(vec![32u32])),
                Arc::new(UInt64Array::from(vec![64u64])),
                Arc::new(Float32Array::from(vec![1.5f32])),
                Arc::new(Float64Array::from(vec![2.25f64])),
                Arc::new(BooleanArray::from(vec![true])),
            ],
        )
        .unwrap();

        let (text, rows) = process_parquet_file(&to_parquet(&batch)).unwrap();

        assert_eq!(rows, 1);
        assert_eq!(text, "str large -8 -16 -32 -64 8 16 32 64 1.5 2.25 true\n");
    }

    #[test]
    fn test_extract_value_from_array_falls_back_to_debug_for_unsupported_types() {
        // Date32 is not one of the handled arrow types, so the debug rendering is used.
        let array = arrow::array::Date32Array::from(vec![19_000]);
        let value = extract_value_from_array(&array, 0);

        assert!(!value.is_empty());
        assert!(value.contains("19000") || value.contains("PrimitiveArray"));
    }
}

/// Endpoint-level tests live in their own module: importing `actix_web::test`
/// shadows the built-in `#[test]` attribute, which the pure-function tests above
/// rely on.
#[cfg(test)]
mod endpoint_tests {
    use super::test_support::{multipart_body, sample_parquet, to_parquet, BOUNDARY};
    use super::*;
    use actix_web::{test, App};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    #[actix_web::test]
    async fn test_endpoint_rejects_a_request_with_no_files() {
        let app = test::init_service(App::new().service(convert_parquet_to_txt)).await;

        let req = test::TestRequest::post()
            .uri("/api/parquet-to-txt")
            .insert_header((
                "content-type",
                format!("multipart/form-data; boundary={}", BOUNDARY),
            ))
            .set_payload(multipart_body(&[("other", None, b"ignored")]))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 400);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], "No parquet files provided");
    }

    #[actix_web::test]
    async fn test_endpoint_ignores_zero_byte_files() {
        let app = test::init_service(App::new().service(convert_parquet_to_txt)).await;

        let req = test::TestRequest::post()
            .uri("/api/parquet-to-txt")
            .insert_header((
                "content-type",
                format!("multipart/form-data; boundary={}", BOUNDARY),
            ))
            .set_payload(multipart_body(&[("files", Some("empty.parquet"), b"")]))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 400);
    }

    #[actix_web::test]
    async fn test_endpoint_streams_the_extracted_text_as_a_download() {
        let app = test::init_service(App::new().service(convert_parquet_to_txt)).await;
        let parquet = sample_parquet();

        let req = test::TestRequest::post()
            .uri("/api/parquet-to-txt")
            .insert_header((
                "content-type",
                format!("multipart/form-data; boundary={}", BOUNDARY),
            ))
            .set_payload(multipart_body(&[("files", Some("data.parquet"), &parquet)]))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "text/plain; charset=utf-8"
        );
        let disposition = resp
            .headers()
            .get("content-disposition")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(disposition.starts_with("attachment; filename=\"imatrix_quantization_data_"));
        assert!(disposition.ends_with(".txt\""));

        let body = test::read_body(resp).await;
        assert_eq!(
            String::from_utf8(body.to_vec()).unwrap(),
            "hello world 1\nsecond row\n3\n"
        );
    }

    #[actix_web::test]
    async fn test_endpoint_concatenates_multiple_files_in_order() {
        let app = test::init_service(App::new().service(convert_parquet_to_txt)).await;

        let schema = Arc::new(Schema::new(vec![Field::new("t", DataType::Utf8, false)]));
        let first = to_parquet(
            &RecordBatch::try_new(
                schema.clone(),
                vec![Arc::new(StringArray::from(vec!["first file"]))],
            )
            .unwrap(),
        );
        let second = to_parquet(
            &RecordBatch::try_new(
                schema,
                vec![Arc::new(StringArray::from(vec!["second file"]))],
            )
            .unwrap(),
        );

        let req = test::TestRequest::post()
            .uri("/api/parquet-to-txt")
            .insert_header((
                "content-type",
                format!("multipart/form-data; boundary={}", BOUNDARY),
            ))
            .set_payload(multipart_body(&[
                ("files", Some("a.parquet"), &first),
                ("files", Some("b.parquet"), &second),
            ]))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
        assert_eq!(body, "first file\nsecond file\n");
    }

    #[actix_web::test]
    async fn test_endpoint_skips_files_without_a_parquet_extension() {
        let app = test::init_service(App::new().service(convert_parquet_to_txt)).await;
        let parquet = sample_parquet();

        // The .txt file is accepted into the batch but produces no output.
        let req = test::TestRequest::post()
            .uri("/api/parquet-to-txt")
            .insert_header((
                "content-type",
                format!("multipart/form-data; boundary={}", BOUNDARY),
            ))
            .set_payload(multipart_body(&[
                ("files", Some("notes.txt"), b"not parquet at all"),
                ("files", Some("data.parquet"), &parquet),
            ]))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
        assert_eq!(body, "hello world 1\nsecond row\n3\n");
    }

    #[actix_web::test]
    async fn test_endpoint_names_a_file_field_without_a_filename() {
        let app = test::init_service(App::new().service(convert_parquet_to_txt)).await;
        let parquet = sample_parquet();

        // No filename in the content disposition: the handler synthesizes
        // "file_0.parquet", which still passes the extension check.
        let req = test::TestRequest::post()
            .uri("/api/parquet-to-txt")
            .insert_header((
                "content-type",
                format!("multipart/form-data; boundary={}", BOUNDARY),
            ))
            .set_payload(multipart_body(&[("files", None, &parquet)]))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status().as_u16(), 200);
        let body = String::from_utf8(test::read_body(resp).await.to_vec()).unwrap();
        assert_eq!(body, "hello world 1\nsecond row\n3\n");
    }
}

/// Fixtures shared by the pure-function and endpoint test modules.
#[cfg(test)]
mod test_support {
    use arrow::array::{Int32Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use parquet::arrow::ArrowWriter;
    use std::sync::Arc;

    pub(super) const BOUNDARY: &str = "----------------parquettest";

    /// Serialize a record batch into an in-memory parquet file.
    pub(super) fn to_parquet(batch: &RecordBatch) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), None).unwrap();
        writer.write(batch).unwrap();
        writer.close().unwrap();
        buffer
    }

    /// A two-column, three-row parquet file with a null in the middle row.
    pub(super) fn sample_parquet() -> Vec<u8> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("text", DataType::Utf8, true),
            Field::new("count", DataType::Int32, true),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec![
                    Some("hello world"),
                    Some("second row"),
                    None,
                ])),
                Arc::new(Int32Array::from(vec![Some(1), None, Some(3)])),
            ],
        )
        .unwrap();
        to_parquet(&batch)
    }

    pub(super) fn multipart_body(parts: &[(&str, Option<&str>, &[u8])]) -> Vec<u8> {
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
}
