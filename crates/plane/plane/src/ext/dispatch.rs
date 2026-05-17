use chamuka_common::ext::network::NetworkPolicy;
use plane_common::protocol::{
    CredentialPayloadWire, PushCredentialWire, PushNetworkPolicyWire, SnapshotCommandWire,
};

pub fn build_push_network_policy(
    backend_id: &str,
    policy: &NetworkPolicy,
    version: u64,
) -> PushNetworkPolicyWire {
    PushNetworkPolicyWire {
        action_id: String::new(),
        backend_id: backend_id.to_string(),
        policy: serde_json::to_value(policy).unwrap_or_default(),
        version,
    }
}

pub fn build_push_credential(
    backend_id: &str,
    cred_id: &str,
    destination: &str,
    header_name: &str,
    header_format: &str,
    value: &str,
) -> PushCredentialWire {
    PushCredentialWire {
        action_id: String::new(),
        backend_id: backend_id.to_string(),
        binding: CredentialPayloadWire {
            id: cred_id.to_string(),
            destination: destination.to_string(),
            header_name: header_name.to_string(),
            header_format: header_format.to_string(),
            value: value.to_string(),
        },
    }
}

pub fn build_snapshot_command(
    action_id: &str,
    backend_id: &str,
    ttl_days: Option<u32>,
    name: Option<&str>,
) -> SnapshotCommandWire {
    SnapshotCommandWire {
        action_id: action_id.to_string(),
        backend_id: backend_id.to_string(),
        ttl_days,
        name: name.map(String::from),
    }
}
