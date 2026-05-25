use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UsageRecord {
    pub tenant: String,
    pub sandbox_id: String,
    pub bucket_start: DateTime<Utc>,
    pub cpu_nsec: u64,
    pub memory_mb_seconds: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}
