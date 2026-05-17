use crate::snapshot_store::SnapshotStore;
use crate::translate::translate_policy;
use common::ext::network::NetworkPolicy;
use serde::Deserialize;
use std::path::Path;
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;

#[derive(Debug, Deserialize)]
pub struct PolicyUpdate {
    pub sandbox_id: String,
    pub policy: NetworkPolicy,
    pub version: u64,
}

#[derive(Debug, Deserialize)]
pub struct PolicyRemove {
    pub sandbox_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DroneMessage {
    UpdatePolicy(PolicyUpdate),
    RemovePolicy(PolicyRemove),
}

pub struct UdsReceiver {
    store: SnapshotStore,
}

impl UdsReceiver {
    pub fn new(store: SnapshotStore) -> Self {
        Self { store }
    }

    pub fn handle_message(&self, msg: DroneMessage) {
        match msg {
            DroneMessage::UpdatePolicy(update) => {
                let snapshot = translate_policy(&update.sandbox_id, &update.policy, update.version);
                self.store.update(&update.sandbox_id, snapshot);
            }
            DroneMessage::RemovePolicy(remove) => {
                self.store.remove(&remove.sandbox_id);
            }
        }
    }

    pub async fn serve(self, socket_path: &Path) -> Result<(), std::io::Error> {
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        let listener = UnixListener::bind(socket_path)?;
        tracing::info!(path = %socket_path.display(), "UDS receiver listening");

        loop {
            let (mut stream, _addr) = listener.accept().await?;
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await?;

            match serde_json::from_slice::<DroneMessage>(&buf) {
                Ok(msg) => self.handle_message(msg),
                Err(err) => {
                    tracing::warn!(?err, "Failed to parse drone message");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ext::network::NetworkPolicyMode;

    #[test]
    fn handle_update_policy_stores_snapshot() {
        let store = SnapshotStore::new();
        let receiver = UdsReceiver::new(store.clone());

        let msg = DroneMessage::UpdatePolicy(PolicyUpdate {
            sandbox_id: "sb-uds-001".to_string(),
            policy: NetworkPolicy {
                mode: NetworkPolicyMode::DenyAll,
                rules: None,
            },
            version: 1,
        });

        receiver.handle_message(msg);

        assert!(store.get("sb-uds-001").is_some());
        assert_eq!(store.sandbox_count(), 1);
    }

    #[test]
    fn handle_remove_policy_clears_snapshot() {
        let store = SnapshotStore::new();
        let receiver = UdsReceiver::new(store.clone());

        receiver.handle_message(DroneMessage::UpdatePolicy(PolicyUpdate {
            sandbox_id: "sb-uds-002".to_string(),
            policy: NetworkPolicy {
                mode: NetworkPolicyMode::AllowAll,
                rules: None,
            },
            version: 1,
        }));
        assert_eq!(store.sandbox_count(), 1);

        receiver.handle_message(DroneMessage::RemovePolicy(PolicyRemove {
            sandbox_id: "sb-uds-002".to_string(),
        }));
        assert_eq!(store.sandbox_count(), 0);
    }

    #[test]
    fn deserialize_update_policy() {
        let json = r#"{
            "type": "update_policy",
            "sandbox_id": "sb-json-001",
            "policy": { "mode": "deny-all", "rules": null },
            "version": 3
        }"#;

        let msg: DroneMessage = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        match msg {
            DroneMessage::UpdatePolicy(update) => {
                assert_eq!(update.sandbox_id, "sb-json-001");
                assert_eq!(update.version, 3);
            }
            _ => panic!("Expected UpdatePolicy variant"),
        }
    }

    #[test]
    fn deserialize_remove_policy() {
        let json = r#"{
            "type": "remove_policy",
            "sandbox_id": "sb-json-002"
        }"#;

        let msg: DroneMessage = serde_json::from_str(json).unwrap_or_else(|e| panic!("{e}"));
        match msg {
            DroneMessage::RemovePolicy(remove) => {
                assert_eq!(remove.sandbox_id, "sb-json-002");
            }
            _ => panic!("Expected RemovePolicy variant"),
        }
    }
}
