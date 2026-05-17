#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::MoveRequest;
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

#[tokio::test]
async fn fs_mkdir_creates_directory() {
    let addr = agentd::serve_on_ephemeral().await;
    let dir = tempfile::tempdir().expect("create tempdir");
    let new_dir = dir.path().join("sub/nested");
    let path_str = new_dir.to_str().unwrap();

    let url = format!("http://{addr}/v1/fs/mkdir");
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({"path": path_str, "recursive": true}))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);
    assert!(new_dir.is_dir());
}

#[tokio::test]
async fn fs_rm_removes_file() {
    let addr = agentd::serve_on_ephemeral().await;

    let mut tmp = NamedTempFile::new().expect("create temp file");
    write!(tmp, "delete me").expect("write");
    let path_str = tmp.path().to_str().unwrap().to_string();

    let url = format!("http://{addr}/v1/fs/rm?path={path_str}");
    let client = reqwest::Client::new();
    let resp = client.delete(&url).send().await.expect("request failed");

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(!std::path::Path::new(&path_str).exists());
}

#[tokio::test]
async fn fs_rm_removes_directory_recursive() {
    let addr = agentd::serve_on_ephemeral().await;
    let dir = tempfile::tempdir().expect("create tempdir");
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).expect("create sub");
    std::fs::write(sub.join("file.txt"), "data").expect("write");
    let path_str = sub.to_str().unwrap();

    let url = format!("http://{addr}/v1/fs/rm?path={path_str}&recursive=true");
    let client = reqwest::Client::new();
    let resp = client.delete(&url).send().await.expect("request failed");

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(!sub.exists());
}

#[tokio::test]
async fn fs_list_returns_entries() {
    let addr = agentd::serve_on_ephemeral().await;
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(dir.path().join("a.txt"), "aaa").expect("write a");
    std::fs::write(dir.path().join("b.txt"), "bbb").expect("write b");
    std::fs::create_dir(dir.path().join("subdir")).expect("create subdir");

    let path_str = dir.path().to_str().unwrap();
    let url = format!("http://{addr}/v1/fs/list?path={path_str}");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);

    let body: common::ext::agentd::ListResponse = resp.json().await.expect("json");
    assert_eq!(body.entries.len(), 3);

    let names: Vec<&str> = body.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"a.txt"));
    assert!(names.contains(&"b.txt"));
    assert!(names.contains(&"subdir"));
}

#[tokio::test]
async fn fs_move_renames_file() {
    let addr = agentd::serve_on_ephemeral().await;
    let dir = tempfile::tempdir().expect("create tempdir");
    let src = dir.path().join("original.txt");
    let dst = dir.path().join("renamed.txt");
    std::fs::write(&src, "move me").expect("write src");

    let url = format!("http://{addr}/v1/fs/move");
    let client = reqwest::Client::new();
    let req = MoveRequest {
        source: src.to_str().unwrap().to_string(),
        destination: dst.to_str().unwrap().to_string(),
    };

    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(!src.exists());
    assert!(dst.exists());
    assert_eq!(std::fs::read_to_string(&dst).unwrap(), "move me");
}

#[tokio::test]
async fn fs_move_nonexistent_returns_not_found() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/fs/move");
    let client = reqwest::Client::new();
    let req = MoveRequest {
        source: "/nonexistent/source".to_string(),
        destination: "/tmp/doesnt-matter".to_string(),
    };

    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn fs_read_with_offset_and_length() {
    let addr = agentd::serve_on_ephemeral().await;
    let mut tmp = NamedTempFile::new().expect("create temp file");
    write!(tmp, "0123456789abcdef").expect("write");
    let path = tmp.path().to_str().unwrap();

    let url = format!("http://{addr}/v1/fs/read?path={path}&offset=4&length=6");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.bytes().await.expect("body");
    assert_eq!(body.as_ref(), b"456789");
}
