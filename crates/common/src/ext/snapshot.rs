use std::collections::HashMap;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::sandbox::Arch;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Snapshot {
    pub id: String,
    pub sandbox_id: String,
    pub tenant: String,
    pub image_ref: String,
    pub digest: String,
    pub base_image: String,
    pub size_bytes: u64,
    pub arch: Arch,
    pub runtime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub tags: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
