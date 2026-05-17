use crate::translate::XdsSnapshot;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SnapshotStore {
    inner: Arc<Mutex<HashMap<String, XdsSnapshot>>>,
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn update(&self, sandbox_id: &str, snapshot: XdsSnapshot) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(sandbox_id.to_string(), snapshot);
    }

    pub fn remove(&self, sandbox_id: &str) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(sandbox_id);
    }

    pub fn get(&self, sandbox_id: &str) -> Option<XdsSnapshot> {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.get(sandbox_id).cloned()
    }

    pub fn sandbox_count(&self) -> usize {
        let map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        map.len()
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
}
