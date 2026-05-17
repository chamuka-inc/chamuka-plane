use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use axum::response::IntoResponse;
use common::ext::agentd::{FsEvent, FsEventKind};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::Deserialize;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[derive(Deserialize)]
pub struct WatchQuery {
    path: String,
    #[serde(default = "default_true")]
    recursive: bool,
}

fn default_true() -> bool {
    true
}

pub async fn watch(Query(q): Query<WatchQuery>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_watch(socket, q.path, q.recursive))
}

async fn handle_watch(mut socket: WebSocket, path: String, recursive: bool) {
    let (tx, mut rx) = mpsc::channel::<FsEvent>(256);

    let mode = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };

    let watch_path = PathBuf::from(&path);
    let sender = tx;
    let _watcher = match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            let kind = match event.kind {
                EventKind::Create(_) => FsEventKind::Create,
                EventKind::Modify(_) => FsEventKind::Modify,
                EventKind::Remove(_) => FsEventKind::Delete,
                _ => return,
            };

            for event_path in &event.paths {
                let is_dir = event_path.is_dir();
                let fs_event = FsEvent {
                    event: kind,
                    path: event_path.to_string_lossy().into_owned(),
                    is_dir,
                    at: timestamp_now(),
                };
                let _ = sender.try_send(fs_event);
            }
        }
    })
    .and_then(|mut w| {
        w.watch(&watch_path, mode)?;
        Ok(w)
    }) {
        Ok(w) => w,
        Err(_) => {
            let _ = socket
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 1011,
                    reason: "failed to start filesystem watcher".into(),
                })))
                .await;
            return;
        }
    };

    while let Some(event) = rx.recv().await {
        let json = match serde_json::to_string(&event) {
            Ok(j) => j,
            Err(_) => break,
        };
        if socket.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }
}

fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}
