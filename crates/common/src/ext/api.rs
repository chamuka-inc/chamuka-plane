use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::network::NetworkPolicy;
use super::sandbox::{Arch, Sandbox, SandboxResources, TrustLevel};
use super::snapshot::Snapshot;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResourcesSpec {
    #[serde(default = "default_vcpus")]
    pub vcpus: u32,
}

fn default_vcpus() -> u32 {
    2
}

impl Default for ResourcesSpec {
    fn default() -> Self {
        Self {
            vcpus: default_vcpus(),
        }
    }
}

impl ResourcesSpec {
    pub fn memory_mb(&self) -> u32 {
        self.vcpus * 2048
    }

    pub fn to_sandbox_resources(&self) -> SandboxResources {
        SandboxResources {
            vcpus: self.vcpus,
            memory_mb: self.memory_mb(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GitSource {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SandboxSource {
    Git(GitSource),
    Snapshot { snapshot_id: String },
    Image { image: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KeySpec {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateSandboxRequest {
    pub runtime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default = "default_trust")]
    pub trust: TrustLevel,
    #[serde(default = "default_arch")]
    pub arch: Arch,
    #[serde(default)]
    pub resources: ResourcesSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SandboxSource>,
    #[serde(default)]
    pub ports: Vec<u16>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
    #[serde(default)]
    pub persistent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_network_policy: Option<NetworkPolicy>,
    #[serde(default)]
    pub use_static_token: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<KeySpec>,
}

fn default_trust() -> TrustLevel {
    TrustLevel::Untrusted
}

fn default_arch() -> Arch {
    Arch::Amd64
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SandboxConnection {
    pub token: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_token: Option<String>,
    pub status_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateSandboxResponse {
    pub sandbox: Sandbox,
    pub connection: SandboxConnection,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetSandboxResponse {
    pub sandbox: Sandbox,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection: Option<SandboxConnection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSandboxesResponse {
    pub items: Vec<Sandbox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_days: Option<u32>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotResponse {
    pub snapshot: Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExtendTimeoutRequest {
    pub additional_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateSecretRequest {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateSecretResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateCredentialBindingRequest {
    pub secret_id: String,
    pub destination: String,
    pub header_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CredentialBinding {
    pub id: String,
    pub sandbox_id: String,
    pub destination: String,
    pub header_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_format: Option<String>,
    pub secret_id: String,
    pub created_at: String,
}
