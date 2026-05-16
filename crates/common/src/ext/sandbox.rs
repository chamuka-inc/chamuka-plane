use std::collections::HashMap;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::types::BackendStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TrustLevel {
    Trusted,
    Untrusted,
    Paranoid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    Amd64,
    Arm64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SandboxResources {
    pub vcpus: u32,
    pub memory_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Sandbox {
    pub id: String,
    pub backend_id: String,
    pub tenant: String,
    pub status: BackendStatus,
    pub runtime: String,
    pub trust: TrustLevel,
    pub arch: Arch,
    pub image: String,
    pub resources: SandboxResources,
    pub ports: Vec<u16>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
    pub persistent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_source: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drone_name: Option<String>,
    pub cluster: String,
}
