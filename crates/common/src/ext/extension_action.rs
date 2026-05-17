use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::network::NetworkPolicy;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PushNetworkPolicyMessage {
    pub backend_id: String,
    pub policy: NetworkPolicy,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PushCredentialMessage {
    pub backend_id: String,
    pub binding: CredentialPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CredentialPayload {
    pub id: String,
    pub destination: String,
    pub header_name: String,
    pub header_format: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SnapshotCommand {
    pub action_id: String,
    pub backend_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_days: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AckExtensionAction {
    pub action_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionActionKind {
    PushPolicy,
    PushCredential,
    RevokeCredential,
    Snapshot,
}

impl std::fmt::Display for ExtensionActionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PushPolicy => f.write_str("push_policy"),
            Self::PushCredential => f.write_str("push_credential"),
            Self::RevokeCredential => f.write_str("revoke_credential"),
            Self::Snapshot => f.write_str("snapshot"),
        }
    }
}
