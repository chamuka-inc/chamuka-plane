use plane_common::protocol::{
    MessageFromDrone, PushCredentialWire, PushNetworkPolicyWire, SnapshotCommandWire,
};
use plane_common::typed_socket::TypedSocketSender;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait EgressPublisher: Send + Sync + 'static {
    async fn push_network_policy(
        &self,
        backend_id: &str,
        policy: &serde_json::Value,
        version: u64,
    ) -> Result<(), anyhow::Error>;

    async fn push_credential(
        &self,
        backend_id: &str,
        binding_id: &str,
        destination: &str,
        header_name: &str,
        header_format: &str,
        value: &str,
    ) -> Result<(), anyhow::Error>;

    async fn revoke_credential(
        &self,
        backend_id: &str,
        binding_id: &str,
    ) -> Result<(), anyhow::Error>;
}

#[async_trait::async_trait]
pub trait SnapshotRuntime: Send + Sync + 'static {
    async fn snapshot(
        &self,
        backend_id: &str,
        ttl_days: Option<u32>,
        name: Option<&str>,
    ) -> Result<SnapshotResult, anyhow::Error>;
}

pub struct SnapshotResult {
    pub image_ref: String,
    pub digest: String,
    pub size_bytes: u64,
}

pub struct ExtensionHandler {
    egress_publisher: Arc<dyn EgressPublisher>,
    snapshot_runtime: Arc<dyn SnapshotRuntime>,
    processed_actions: Arc<Mutex<HashMap<String, ()>>>,
}

impl ExtensionHandler {
    pub fn new(
        egress_publisher: Arc<dyn EgressPublisher>,
        snapshot_runtime: Arc<dyn SnapshotRuntime>,
    ) -> Self {
        Self {
            egress_publisher,
            snapshot_runtime,
            processed_actions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn already_processed(&self, action_id: &str) -> bool {
        let mut map = self
            .processed_actions
            .lock()
            .expect("Processed-actions lock poisoned");
        if map.contains_key(action_id) {
            return true;
        }
        map.insert(action_id.to_string(), ());
        false
    }

    pub async fn handle_push_network_policy(
        &self,
        wire: PushNetworkPolicyWire,
        sender: &TypedSocketSender<MessageFromDrone>,
    ) {
        if self.already_processed(&wire.action_id) {
            tracing::debug!(action_id = %wire.action_id, "Duplicate PushNetworkPolicy; re-acking");
            self.ack(sender, &wire.action_id);
            return;
        }

        match self
            .egress_publisher
            .push_network_policy(&wire.backend_id, &wire.policy, wire.version)
            .await
        {
            Ok(()) => {
                tracing::info!(
                    backend_id = %wire.backend_id,
                    version = wire.version,
                    "Network policy pushed to egress publisher"
                );
                self.ack(sender, &wire.action_id);
            }
            Err(err) => {
                tracing::error!(?err, backend_id = %wire.backend_id, "Failed to push network policy");
            }
        }
    }

    pub async fn handle_push_credential(
        &self,
        wire: PushCredentialWire,
        sender: &TypedSocketSender<MessageFromDrone>,
    ) {
        if self.already_processed(&wire.action_id) {
            tracing::debug!(action_id = %wire.action_id, "Duplicate PushCredential; re-acking");
            self.ack(sender, &wire.action_id);
            return;
        }

        match self
            .egress_publisher
            .push_credential(
                &wire.backend_id,
                &wire.binding.id,
                &wire.binding.destination,
                &wire.binding.header_name,
                &wire.binding.header_format,
                &wire.binding.value,
            )
            .await
        {
            Ok(()) => {
                tracing::info!(
                    backend_id = %wire.backend_id,
                    binding_id = %wire.binding.id,
                    "Credential pushed to egress publisher"
                );
                self.ack(sender, &wire.action_id);
            }
            Err(err) => {
                tracing::error!(?err, backend_id = %wire.backend_id, "Failed to push credential");
            }
        }
    }

    pub async fn handle_revoke_credential(
        &self,
        action_id: &str,
        binding_id: &str,
        sender: &TypedSocketSender<MessageFromDrone>,
    ) {
        if self.already_processed(action_id) {
            tracing::debug!(action_id = %action_id, "Duplicate RevokeCredential; re-acking");
            self.ack(sender, action_id);
            return;
        }

        match self
            .egress_publisher
            .revoke_credential("", binding_id)
            .await
        {
            Ok(()) => {
                tracing::info!(binding_id = %binding_id, "Credential revoked in egress publisher");
                self.ack(sender, action_id);
            }
            Err(err) => {
                tracing::error!(?err, binding_id = %binding_id, "Failed to revoke credential");
            }
        }
    }

    pub async fn handle_snapshot(
        &self,
        wire: SnapshotCommandWire,
        sender: &TypedSocketSender<MessageFromDrone>,
    ) {
        if self.already_processed(&wire.action_id) {
            tracing::debug!(action_id = %wire.action_id, "Duplicate Snapshot; re-acking");
            self.ack(sender, &wire.action_id);
            return;
        }

        match self
            .snapshot_runtime
            .snapshot(&wire.backend_id, wire.ttl_days, wire.name.as_deref())
            .await
        {
            Ok(result) => {
                tracing::info!(
                    backend_id = %wire.backend_id,
                    image_ref = %result.image_ref,
                    digest = %result.digest,
                    size_bytes = result.size_bytes,
                    "Snapshot complete"
                );
                self.ack(sender, &wire.action_id);
            }
            Err(err) => {
                tracing::error!(
                    ?err,
                    backend_id = %wire.backend_id,
                    action_id = %wire.action_id,
                    "Snapshot failed"
                );
            }
        }
    }

    fn ack(&self, sender: &TypedSocketSender<MessageFromDrone>, action_id: &str) {
        if let Err(err) = sender.send(MessageFromDrone::AckExtensionAction {
            action_id: action_id.to_string(),
        }) {
            tracing::error!(?err, action_id = %action_id, "Failed to send extension action ack");
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use plane_common::typed_socket::SocketAction;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockEgressPublisher {
        policy_calls: AtomicUsize,
        credential_calls: AtomicUsize,
        revoke_calls: AtomicUsize,
    }

    impl MockEgressPublisher {
        fn new() -> Self {
            Self {
                policy_calls: AtomicUsize::new(0),
                credential_calls: AtomicUsize::new(0),
                revoke_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl EgressPublisher for MockEgressPublisher {
        async fn push_network_policy(
            &self,
            _backend_id: &str,
            _policy: &serde_json::Value,
            _version: u64,
        ) -> Result<(), anyhow::Error> {
            self.policy_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn push_credential(
            &self,
            _backend_id: &str,
            _binding_id: &str,
            _destination: &str,
            _header_name: &str,
            _header_format: &str,
            _value: &str,
        ) -> Result<(), anyhow::Error> {
            self.credential_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn revoke_credential(
            &self,
            _backend_id: &str,
            _binding_id: &str,
        ) -> Result<(), anyhow::Error> {
            self.revoke_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct MockSnapshotRuntime {
        snapshot_calls: AtomicUsize,
    }

    impl MockSnapshotRuntime {
        fn new() -> Self {
            Self {
                snapshot_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl SnapshotRuntime for MockSnapshotRuntime {
        async fn snapshot(
            &self,
            _backend_id: &str,
            _ttl_days: Option<u32>,
            _name: Option<&str>,
        ) -> Result<SnapshotResult, anyhow::Error> {
            self.snapshot_calls.fetch_add(1, Ordering::SeqCst);
            Ok(SnapshotResult {
                image_ref: "registry.example.com/snap:abc123".to_string(),
                digest: "sha256:abcdef1234567890".to_string(),
                size_bytes: 1024 * 1024 * 50,
            })
        }
    }

    fn make_test_sender() -> (
        TypedSocketSender<MessageFromDrone>,
        Arc<Mutex<Vec<MessageFromDrone>>>,
    ) {
        let messages: Arc<Mutex<Vec<MessageFromDrone>>> = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();
        let sender = TypedSocketSender::from_fn(move |action: SocketAction<MessageFromDrone>| {
            if let SocketAction::Send(msg) = action {
                messages_clone.lock().expect("lock").push(msg);
            }
            Ok(())
        });
        (sender, messages)
    }

    fn make_handler() -> (
        Arc<MockEgressPublisher>,
        Arc<MockSnapshotRuntime>,
        ExtensionHandler,
    ) {
        let publisher = Arc::new(MockEgressPublisher::new());
        let snapshot_rt = Arc::new(MockSnapshotRuntime::new());
        let handler = ExtensionHandler::new(publisher.clone(), snapshot_rt.clone());
        (publisher, snapshot_rt, handler)
    }

    #[tokio::test]
    async fn push_network_policy_calls_publisher_and_acks() {
        let (publisher, _snapshot_rt, handler) = make_handler();
        let (sender, messages) = make_test_sender();

        let wire = PushNetworkPolicyWire {
            action_id: "xak-p1".to_string(),
            backend_id: "bk-test".to_string(),
            policy: serde_json::json!({"mode": "deny-all"}),
            version: 2,
        };

        handler.handle_push_network_policy(wire, &sender).await;

        assert_eq!(publisher.policy_calls.load(Ordering::SeqCst), 1);
        let msgs = messages.lock().expect("lock");
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            MessageFromDrone::AckExtensionAction { action_id } => {
                assert_eq!(action_id, "xak-p1");
            }
            other => panic!("Expected AckExtensionAction, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn push_credential_calls_publisher_and_acks() {
        let (publisher, _snapshot_rt, handler) = make_handler();
        let (sender, messages) = make_test_sender();

        let wire = PushCredentialWire {
            action_id: "xak-c1".to_string(),
            backend_id: "bk-test".to_string(),
            binding: plane_common::protocol::CredentialPayloadWire {
                id: "cred-001".to_string(),
                destination: "api.openai.com".to_string(),
                header_name: "Authorization".to_string(),
                header_format: "Bearer {value}".to_string(),
                value: "sk-test123".to_string(),
            },
        };

        handler.handle_push_credential(wire, &sender).await;

        assert_eq!(publisher.credential_calls.load(Ordering::SeqCst), 1);
        let msgs = messages.lock().expect("lock");
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            MessageFromDrone::AckExtensionAction { action_id } => {
                assert_eq!(action_id, "xak-c1");
            }
            other => panic!("Expected AckExtensionAction, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn revoke_credential_calls_publisher_and_acks() {
        let (publisher, _snapshot_rt, handler) = make_handler();
        let (sender, messages) = make_test_sender();

        handler
            .handle_revoke_credential("xak-r1", "cred-001", &sender)
            .await;

        assert_eq!(publisher.revoke_calls.load(Ordering::SeqCst), 1);
        let msgs = messages.lock().expect("lock");
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            MessageFromDrone::AckExtensionAction { action_id } => {
                assert_eq!(action_id, "xak-r1");
            }
            other => panic!("Expected AckExtensionAction, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn snapshot_calls_runtime_and_acks() {
        let (_publisher, snapshot_rt, handler) = make_handler();
        let (sender, messages) = make_test_sender();

        let wire = SnapshotCommandWire {
            action_id: "xak-s1".to_string(),
            backend_id: "bk-test".to_string(),
            ttl_days: Some(30),
            name: Some("pre-deploy".to_string()),
        };

        handler.handle_snapshot(wire, &sender).await;

        assert_eq!(snapshot_rt.snapshot_calls.load(Ordering::SeqCst), 1);
        let msgs = messages.lock().expect("lock");
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            MessageFromDrone::AckExtensionAction { action_id } => {
                assert_eq!(action_id, "xak-s1");
            }
            other => panic!("Expected AckExtensionAction, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn duplicate_action_is_idempotent() {
        let (publisher, _snapshot_rt, handler) = make_handler();
        let (sender, messages) = make_test_sender();

        let wire = PushNetworkPolicyWire {
            action_id: "xak-dup".to_string(),
            backend_id: "bk-test".to_string(),
            policy: serde_json::json!({"mode": "allow-all"}),
            version: 1,
        };

        handler
            .handle_push_network_policy(wire.clone(), &sender)
            .await;
        handler.handle_push_network_policy(wire, &sender).await;

        // Publisher only called once; ack sent twice (idempotent)
        assert_eq!(publisher.policy_calls.load(Ordering::SeqCst), 1);
        let msgs = messages.lock().expect("lock");
        assert_eq!(msgs.len(), 2);
    }
}
