use crate::translate::XdsSnapshot;
use envoy_types::pb::google::protobuf::Any;
use prost::Message;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

struct Inner {
    snapshots: HashMap<String, XdsSnapshot>,
    generation: u64,
}

#[derive(Clone)]
pub struct SnapshotStore {
    inner: Arc<Mutex<Inner>>,
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                snapshots: HashMap::new(),
                generation: 0,
            })),
        }
    }

    pub fn update(&self, sandbox_id: &str, snapshot: XdsSnapshot) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.snapshots.insert(sandbox_id.to_string(), snapshot);
        inner.generation += 1;
    }

    pub fn remove(&self, sandbox_id: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.snapshots.remove(sandbox_id).is_some() {
            inner.generation += 1;
        }
    }

    pub fn get(&self, sandbox_id: &str) -> Option<XdsSnapshot> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.snapshots.get(sandbox_id).cloned()
    }

    pub fn sandbox_count(&self) -> usize {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.snapshots.len()
    }

    pub fn generation(&self) -> u64 {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.generation
    }

    pub fn collect_resources(&self, type_url: &str) -> Vec<Any> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let type_url_str = type_url.to_string();
        let mut resources = Vec::with_capacity(inner.snapshots.len());

        for snapshot in inner.snapshots.values() {
            match type_url {
                super::xds_server::TYPE_URL_LDS => {
                    for listener in &snapshot.listeners {
                        resources.push(Any {
                            type_url: type_url_str.clone(),
                            value: listener.encode_to_vec(),
                        });
                    }
                }
                super::xds_server::TYPE_URL_RDS => {
                    for route in &snapshot.routes {
                        resources.push(Any {
                            type_url: type_url_str.clone(),
                            value: route.encode_to_vec(),
                        });
                    }
                }
                super::xds_server::TYPE_URL_CDS => {
                    for cluster in &snapshot.clusters {
                        resources.push(Any {
                            type_url: type_url_str.clone(),
                            value: cluster.encode_to_vec(),
                        });
                    }
                }
                _ => {}
            }
        }

        resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ext::network::{NetworkPolicy, NetworkPolicyMode};

    fn make_snapshot(sandbox_id: &str) -> XdsSnapshot {
        crate::translate::translate_policy(
            sandbox_id,
            &NetworkPolicy {
                mode: NetworkPolicyMode::DenyAll,
                rules: None,
            },
            1,
        )
    }

    #[test]
    fn store_and_retrieve() {
        let store = SnapshotStore::new();
        let snapshot = make_snapshot("sb-001");
        store.update("sb-001", snapshot);

        let retrieved = store.get("sb-001");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.as_ref().map(|s| s.version.as_str()), Some("1"));
    }

    #[test]
    fn remove_clears_entry() {
        let store = SnapshotStore::new();
        store.update("sb-002", make_snapshot("sb-002"));
        assert_eq!(store.sandbox_count(), 1);

        store.remove("sb-002");
        assert_eq!(store.sandbox_count(), 0);
        assert!(store.get("sb-002").is_none());
    }

    #[test]
    fn update_replaces_existing() {
        let store = SnapshotStore::new();
        store.update("sb-003", make_snapshot("sb-003"));

        let updated = crate::translate::translate_policy(
            "sb-003",
            &NetworkPolicy {
                mode: NetworkPolicyMode::AllowAll,
                rules: None,
            },
            2,
        );
        store.update("sb-003", updated);

        let retrieved = store.get("sb-003");
        assert_eq!(retrieved.as_ref().map(|s| s.version.as_str()), Some("2"));
        assert_eq!(store.sandbox_count(), 1);
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let store = SnapshotStore::new();
        assert!(store.get("sb-nonexistent").is_none());
    }

    #[test]
    fn generation_increments_on_mutation() {
        let store = SnapshotStore::new();
        assert_eq!(store.generation(), 0);

        store.update("sb-gen", make_snapshot("sb-gen"));
        assert_eq!(store.generation(), 1);

        store.update("sb-gen", make_snapshot("sb-gen"));
        assert_eq!(store.generation(), 2);

        store.remove("sb-gen");
        assert_eq!(store.generation(), 3);
    }

    #[test]
    fn remove_nonexistent_does_not_increment() {
        let store = SnapshotStore::new();
        store.remove("sb-nope");
        assert_eq!(store.generation(), 0);
    }
}
