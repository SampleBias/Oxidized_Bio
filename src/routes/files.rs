use axum::{
    extract::{Multipart, State},
    routing::post,
    Json, Router,
};
use crate::data_registry::{DatasetRecord};
use crate::models::{AppState, UploadedDataset};
use bytes::Bytes;
use tokio::fs;
use tracing::{info, warn};
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/files/{*path}", post(upload_file))
        .with_state(state)
}

async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    info!("File upload request received");

    let mut file_bytes: Option<Bytes> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut description: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            filename = field.file_name().map(|s| s.to_string());
            content_type = field.content_type().map(|s| s.to_string());
            file_bytes = Some(field.bytes().await.map_err(|_| axum::http::StatusCode::BAD_REQUEST)?);
        } else if name == "description" {
            description = Some(field.text().await.unwrap_or_default());
        }
    }

    let filename = filename.ok_or(axum::http::StatusCode::BAD_REQUEST)?;
    let file_bytes = file_bytes.ok_or(axum::http::StatusCode::BAD_REQUEST)?;

    let extension = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension != "csv" && extension != "tsv" {
        warn!(filename = %filename, "Rejected unsupported file type");
        return Err(axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    let delimiter = if extension == "tsv" { b'\t' } else { b',' };
    let dataset_id = Uuid::new_v4().to_string();
    let upload_dir = std::path::Path::new("uploads");
    fs::create_dir_all(upload_dir)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    let stored_name = format!("{}-{}", dataset_id, filename);
    let local_path = upload_dir.join(&stored_name);
    fs::write(&local_path, &file_bytes)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let (columns, row_count) = infer_csv_metadata(&file_bytes, delimiter)?;

    let dataset = UploadedDataset {
        filename: filename.clone(),
        id: dataset_id.clone(),
        description: description.unwrap_or_else(|| format!("Uploaded dataset {}", filename)),
        path: Some(local_path.to_string_lossy().to_string()),
        content: None,
        size: Some(file_bytes.len() as i64),
    };

    let record = DatasetRecord {
        dataset: dataset.clone(),
        local_path: local_path.to_string_lossy().to_string(),
        content_type: content_type.unwrap_or_else(|| "text/plain".to_string()),
        delimiter,
        has_headers: true,
        columns: columns.clone(),
        row_count,
    };
    state.dataset_registry.insert(record).await;

    let response = serde_json::json!({
        "status": "success",
        "message": "File uploaded successfully",
        "dataset": {
            "id": dataset.id,
            "filename": dataset.filename,
            "description": dataset.description,
            "size": dataset.size,
            "path": dataset.path,
            "columns": columns,
            "row_count": row_count,
            "delimiter": if delimiter == b'\t' { "tab" } else { "comma" },
        }
    });

    Ok(Json(response))
}

fn infer_csv_metadata(bytes: &Bytes, delimiter: u8) -> Result<(Vec<String>, usize), axum::http::StatusCode> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .from_reader(bytes.as_ref());

    let headers = rdr
        .headers()
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)?
        .iter()
        .map(|h| h.to_string())
        .collect::<Vec<_>>();

    let mut row_count = 0usize;
    for record in rdr.records() {
        record.map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;
        row_count += 1;
    }
    Ok((headers, row_count))
}
