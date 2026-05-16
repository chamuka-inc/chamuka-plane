#![allow(clippy::unwrap_used, clippy::expect_used)]

use reqwest::StatusCode;
use std::io::Write;
use tempfile::NamedTempFile;

#[tokio::test]
async fn fs_read_existing_file() {
    let addr = agentd::serve_on_ephemeral().await;

    let mut tmp = NamedTempFile::new().expect("create temp file");
    write!(tmp, "file contents here").expect("write to temp");
    let path = tmp.path().to_str().unwrap();

    let url = format!("http://{addr}/v1/fs/read?path={path}");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.bytes().await.expect("body");
    assert_eq!(body.as_ref(), b"file contents here");
}

#[tokio::test]
async fn fs_read_nonexistent() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/fs/read?path=/nonexistent/path/abc123");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn fs_write_creates_file() {
    let addr = agentd::serve_on_ephemeral().await;
    let dir = tempfile::tempdir().expect("create tempdir");
    let file_path = dir.path().join("written.txt");
    let path_str = file_path.to_str().unwrap();

    let url = format!("http://{addr}/v1/fs/write");
    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .text("path", path_str.to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"hello world".to_vec()).file_name("written.txt"),
        );

    let resp = client
        .put(&url)
        .multipart(form)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);
    let contents = std::fs::read_to_string(&file_path).expect("read written file");
    assert_eq!(contents, "hello world");
}

#[tokio::test]
async fn fs_write_overwrites_existing() {
    let addr = agentd::serve_on_ephemeral().await;

    let mut tmp = NamedTempFile::new().expect("create temp file");
    write!(tmp, "old content").expect("write");
    let path_str = tmp.path().to_str().unwrap().to_string();

    let url = format!("http://{addr}/v1/fs/write");
    let client = reqwest::Client::new();
    let form = reqwest::multipart::Form::new()
        .text("path", path_str.clone())
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"new content".to_vec()).file_name("f.txt"),
        );

    let resp = client
        .put(&url)
        .multipart(form)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);
    let contents = std::fs::read_to_string(&path_str).expect("read file");
    assert_eq!(contents, "new content");
}
