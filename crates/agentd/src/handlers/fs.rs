use axum::body::Body;
use axum::extract::{Multipart, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
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
