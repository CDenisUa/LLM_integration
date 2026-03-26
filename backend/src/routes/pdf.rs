use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct PdfMetadata {
    pub id: String,
    pub filename: String,
    pub uploaded_at: u64,
    #[serde(default)]
    pub has_cover: bool,
}

#[derive(Deserialize)]
pub struct UpdatePdfRequest {
    pub filename: String,
}

pub async fn list_pdfs() -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mut entries = match fs::read_dir("storage/pdfs").await {
        Ok(read_dir) => read_dir,
        Err(_) => return Ok(Json(vec![] as Vec<PdfMetadata>)),
    };

    let mut pdfs = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "json" {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(metadata) = serde_json::from_str::<PdfMetadata>(&content) {
                        pdfs.push(metadata);
                    }
                }
            }
        }
    }

    pdfs.sort_by(|a, b| b.uploaded_at.cmp(&a.uploaded_at));

    Ok(Json(pdfs))
}

pub async fn update_pdf(
    Path(id): Path<String>,
    Json(payload): Json<UpdatePdfRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let json_path = format!("storage/pdfs/{}.json", id);
    
    let content = fs::read_to_string(&json_path).await.map_err(|_| {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "PDF not found"})))
    })?;

    let mut metadata: PdfMetadata = serde_json::from_str(&content).map_err(|_| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Invalid metadata format"})))
    })?;

    metadata.filename = payload.filename;

    let meta_json = serde_json::to_string(&metadata).unwrap();
    fs::write(&json_path, meta_json).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to write metadata: {}", e)})))
    })?;

    Ok(Json(metadata))
}

pub async fn delete_pdf(
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let pdf_path = format!("storage/pdfs/{}.pdf", id);
    let json_path = format!("storage/pdfs/{}.json", id);
    let cover_path = format!("storage/covers/{}.jpg", id);

    fs::metadata(&json_path).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "PDF not found"})),
        )
    })?;

    fs::remove_file(&pdf_path).await.or_else(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(error)
        }
    }).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to delete pdf file: {}", e)})),
        )
    })?;

    fs::remove_file(&cover_path).await.or_else(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(error)
        }
    }).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to delete cover file: {}", e)})),
        )
    })?;

    fs::remove_file(&json_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to delete metadata: {}", e)})),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn upload_pdf(
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mut filename = "untitled.pdf".to_string();
    let mut file_data = Vec::new();
    let mut cover_data = Vec::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if let Some(name) = field.name() {
            if name == "file" {
                if let Some(file_name) = field.file_name() {
                    filename = file_name.to_string();
                }
                if let Ok(bytes) = field.bytes().await {
                    file_data.extend_from_slice(&bytes);
                }
            } else if name == "cover" {
                if let Ok(bytes) = field.bytes().await {
                    cover_data.extend_from_slice(&bytes);
                }
            }
        }
    }

    if file_data.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "No file mapped or file is empty"})),
        ));
    }

    let id = Uuid::new_v4().to_string();
    
    // Save PDF
    let pdf_path = format!("storage/pdfs/{}.pdf", id);
    let mut pdf_file = File::create(&pdf_path).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to create pdf file: {}", e)})))
    })?;
    pdf_file.write_all(&file_data).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to write pdf file: {}", e)})))
    })?;

    let mut has_cover = false;
    // Save Cover if provided
    if !cover_data.is_empty() {
        let cover_path = format!("storage/covers/{}.jpg", id);
        if let Ok(mut cover_file) = File::create(&cover_path).await {
            let _ = cover_file.write_all(&cover_data).await;
            has_cover = true;
        }
    }

    // Save Metadata
    let uploaded_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let meta = PdfMetadata { id: id.clone(), filename, uploaded_at, has_cover };
    let json_path = format!("storage/pdfs/{}.json", id);
    
    let meta_json = serde_json::to_string(&meta).unwrap();
    fs::write(&json_path, meta_json).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to write metadata: {}", e)})))
    })?;

    Ok(Json(meta))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        path::{Path as StdPath, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestDirGuard {
        original_dir: PathBuf,
        temp_dir: PathBuf,
    }

    impl TestDirGuard {
        fn enter() -> Self {
            let original_dir = env::current_dir().expect("current dir");
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos();
            let temp_dir = env::temp_dir().join(format!("pdf-delete-test-{unique}"));

            std::fs::create_dir_all(temp_dir.join("storage/pdfs")).expect("create pdf storage");
            std::fs::create_dir_all(temp_dir.join("storage/covers")).expect("create cover storage");
            env::set_current_dir(&temp_dir).expect("enter temp dir");

            Self {
                original_dir,
                temp_dir,
            }
        }
    }

    impl Drop for TestDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original_dir);
            let _ = std::fs::remove_dir_all(&self.temp_dir);
        }
    }

    #[tokio::test]
    async fn delete_pdf_removes_pdf_metadata_and_cover() {
        let _guard = TestDirGuard::enter();
        let id = "test-book";

        std::fs::write(format!("storage/pdfs/{id}.pdf"), b"pdf").expect("write pdf");
        std::fs::write(
            format!("storage/pdfs/{id}.json"),
            r#"{"id":"test-book","filename":"Test.pdf","uploaded_at":1,"has_cover":true}"#,
        )
        .expect("write metadata");
        std::fs::write(format!("storage/covers/{id}.jpg"), b"cover").expect("write cover");

        let response = delete_pdf(Path(id.to_string()))
            .await
            .expect("delete should succeed")
            .into_response();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(!StdPath::new(&format!("storage/pdfs/{id}.pdf")).exists());
        assert!(!StdPath::new(&format!("storage/pdfs/{id}.json")).exists());
        assert!(!StdPath::new(&format!("storage/covers/{id}.jpg")).exists());
    }
}
