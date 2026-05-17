use axum::body::Body;
use axum::extract::{Multipart, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use common::ext::agentd::{DirEntry, EntryType, ListResponse, MkdirRequest, MoveRequest};
use serde::Deserialize;
use std::os::unix::fs::MetadataExt;
use tokio::fs;

#[derive(Deserialize)]
pub struct ReadQuery {
    path: String,
    offset: Option<u64>,
    length: Option<u64>,
}

pub async fn read(Query(q): Query<ReadQuery>) -> impl IntoResponse {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    let mut file = match fs::File::open(&q.path).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (StatusCode::NOT_FOUND, Body::from("not found")).into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Body::from(format!("read error: {e}")),
            )
                .into_response();
        }
    };

    if let Some(offset) = q.offset {
        if let Err(e) = file.seek(std::io::SeekFrom::Start(offset)).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Body::from(format!("seek error: {e}")),
            )
                .into_response();
        }
    }

    let data = match q.length {
        Some(len) => {
            let mut buf = vec![0u8; len as usize];
            let n = match file.read(&mut buf).await {
                Ok(n) => n,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Body::from(format!("read error: {e}")),
                    )
                        .into_response();
                }
            };
            buf.truncate(n);
            buf
        }
        None => {
            let mut buf = Vec::new();
            if let Err(e) = file.read_to_end(&mut buf).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Body::from(format!("read error: {e}")),
                )
                    .into_response();
            }
            buf
        }
    };

    (StatusCode::OK, Body::from(data)).into_response()
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
        if let Err(e) = fs::create_dir_all(parent).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to create parent directory: {e}"),
            )
                .into_response();
        }
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
        let file_type = entry.file_type().await;
        let meta = entry.metadata().await;
        let (entry_type, size, mode, modified_at) = match (&file_type, &meta) {
            (Ok(ft), Ok(m)) => {
                let et = if ft.is_symlink() {
                    EntryType::Symlink
                } else if ft.is_dir() {
                    EntryType::Dir
                } else if ft.is_file() {
                    EntryType::File
                } else {
                    EntryType::Other
                };
                let size = if ft.is_file() { Some(m.len()) } else { None };
                let modified = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_default();
                (et, size, m.mode(), modified)
            }
            _ => (EntryType::Other, None, 0, String::new()),
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

pub async fn mv(Json(req): Json<MoveRequest>) -> impl IntoResponse {
    match fs::rename(&req.source, &req.destination).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("move error: {e}"),
        )
            .into_response(),
    }
}
