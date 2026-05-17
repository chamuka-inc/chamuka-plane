use axum::body::Body;
use axum::extract::{Multipart, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::ext::agentd::{DirEntry, EntryType, ListResponse, MkdirRequest};
use serde::Deserialize;
use std::os::unix::fs::MetadataExt;
use tokio::fs;

#[derive(Deserialize)]
pub struct ReadQuery {
    path: String,
    #[allow(dead_code)]
    offset: Option<u64>,
    #[allow(dead_code)]
    length: Option<u64>,
}

pub async fn read(Query(q): Query<ReadQuery>) -> impl IntoResponse {
    match fs::read(&q.path).await {
        Ok(data) => (StatusCode::OK, Body::from(data)).into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            (StatusCode::NOT_FOUND, Body::from("not found")).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Body::from(format!("read error: {e}")),
        )
            .into_response(),
    }
}

pub async fn write(mut multipart: Multipart) -> impl IntoResponse {
    let mut path: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("path") => {
                path = field.text().await.ok();
            }
            Some("file") => {
                file_data = field.bytes().await.ok().map(|b| b.to_vec());
            }
            _ => {}
        }
    }

    let Some(path) = path else {
        return (StatusCode::BAD_REQUEST, "missing 'path' field").into_response();
    };
    let Some(data) = file_data else {
        return (StatusCode::BAD_REQUEST, "missing 'file' field").into_response();
    };

    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = fs::create_dir_all(parent).await;
    }

    match fs::write(&path, &data).await {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("write error: {e}"),
        )
            .into_response(),
    }
}

pub async fn mkdir(Json(req): Json<MkdirRequest>) -> impl IntoResponse {
    let result = if req.recursive {
        fs::create_dir_all(&req.path).await
    } else {
        fs::create_dir(&req.path).await
    };

    match result {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("mkdir error: {e}"),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct RmQuery {
    path: String,
    #[serde(default)]
    recursive: bool,
}

pub async fn rm(Query(q): Query<RmQuery>) -> impl IntoResponse {
    let result = if q.recursive {
        fs::remove_dir_all(&q.path).await
    } else {
        let meta = fs::metadata(&q.path).await;
        match meta {
            Ok(m) if m.is_dir() => fs::remove_dir(&q.path).await,
            Ok(_) => fs::remove_file(&q.path).await,
            Err(e) => Err(e),
        }
    };

    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("rm error: {e}")).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    path: String,
}

pub async fn list(Query(q): Query<ListQuery>) -> impl IntoResponse {
    let mut read_dir = match fs::read_dir(&q.path).await {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (StatusCode::NOT_FOUND, "not found").into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("list error: {e}"),
            )
                .into_response();
        }
    };

    let mut entries = Vec::new();
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().await;
        let (entry_type, size, mode, modified_at) = match meta {
            Ok(m) => {
                let et = if m.is_dir() {
                    EntryType::Dir
                } else if m.is_symlink() {
                    EntryType::Symlink
                } else if m.is_file() {
                    EntryType::File
                } else {
                    EntryType::Other
                };
                let size = if m.is_file() { Some(m.len()) } else { None };
                let modified = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_default();
                (et, size, m.mode(), modified)
            }
            Err(_) => (EntryType::Other, None, 0, String::new()),
        };

        entries.push(DirEntry {
            name,
            entry_type,
            size,
            mode,
            modified_at,
        });
    }

    let resp = ListResponse {
        path: q.path,
        entries,
    };
    (StatusCode::OK, Json(resp)).into_response()
}
