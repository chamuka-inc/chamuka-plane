use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

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

impl fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trusted => f.write_str("trusted"),
            Self::Untrusted => f.write_str("untrusted"),
            Self::Paranoid => f.write_str("paranoid"),
        }
    }
}

impl FromStr for TrustLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trusted" => Ok(Self::Trusted),
            "untrusted" => Ok(Self::Untrusted),
            "paranoid" => Ok(Self::Paranoid),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Arch {
    Amd64,
    Arm64,
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Amd64 => f.write_str("amd64"),
            Self::Arm64 => f.write_str("arm64"),
        }
    }
}

impl FromStr for Arch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "amd64" => Ok(Self::Amd64),
            "arm64" => Ok(Self::Arm64),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SandboxResources {
    pub vcpus: u32,
    pub memory_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Sandbox {
    pub id: String,
    /// Plane backend currently (or most recently) hosting this sandbox.
    /// Null once the backend has been reaped by post-termination cleanup
    /// — the sandbox row itself outlives its backend per spec retention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_id: Option<String>,
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
