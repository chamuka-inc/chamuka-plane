use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecRequest {
    pub cmd: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub detached: bool,
    #[serde(default)]
    pub pty: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pty_size: Option<PtySize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecSyncResponse {
    pub cmd_id: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DetachedStatus {
    Started,
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExecDetachedResponse {
    pub cmd_id: String,
    pub status: DetachedStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "stream", rename_all = "lowercase")]
pub enum LogFrame {
    Stdout { data: String },
    Stderr { data: String },
    Exit { code: i32, signal: Option<i32> },
    Error { code: LogErrorCode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LogErrorCode {
    Backpressure,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Signal {
    Term,
    Kill,
    Int,
    Hup,
    Usr1,
    Usr2,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SignalRequest {
    pub signal: Signal,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MkdirRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<u32>,
    #[serde(default)]
    pub recursive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    File,
    Dir,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DirEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: EntryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    pub mode: u32,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListResponse {
    pub path: String,
    pub entries: Vec<DirEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FsEventKind {
    Create,
    Modify,
    Delete,
    Rename,
    Attrib,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FsEvent {
    pub event: FsEventKind,
    pub path: String,
    pub is_dir: bool,
    pub at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListeningPort {
    pub port: u16,
    pub protocol: PortProtocol,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PortsResponse {
    pub listening: Vec<ListeningPort>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ComputerAction {
    Click,
    DoubleClick,
    RightClick,
    MiddleClick,
    Drag,
    Type,
    KeyPress,
    Scroll,
    Screenshot,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComputerActionRequest {
    pub action: ComputerAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordinate: Option<[i32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_coordinate: Option<[i32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<[i32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_direction: Option<ScrollDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_amount: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ComputerOutput {
    Image { base64: String, media_type: String },
    Text { text: String },
    Ok,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ComputerActionResponse {
    pub output: ComputerOutput,
}
