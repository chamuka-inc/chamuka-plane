use plane_common::protocol::{CredentialPayloadWire, MessageFromDrone, MessageToDrone};
use plane_common::typed_socket::TypedSocketSender;
use std::collections::HashSet;
use std::future::Future;
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
        binding: &CredentialPayloadWire,
    ) -> Result<(), anyhow::Error>;

    async fn revoke_credential(&self, binding_id: &str) -> Result<(), anyhow::Error>;
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
    processed_actions: Arc<Mutex<HashSet<String>>>,
}

impl ExtensionHandler {
    pub fn new(
        egress_publisher: Arc<dyn EgressPublisher>,
        snapshot_runtime: Arc<dyn SnapshotRuntime>,
    ) -> Self {
        Self {
            egress_publisher,
            snapshot_runtime,
            processed_actions: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn clear_processed(&self) {
        self.processed_actions
            .lock()
            .expect("Processed-actions lock poisoned")
            .clear();
    }

    fn already_processed(&self, action_id: &str) -> bool {
        let mut set = self
            .processed_actions
            .lock()
            .expect("Processed-actions lock poisoned");
        !set.insert(action_id.to_string())
    }

    pub async fn dispatch(
        &self,
        message: MessageToDrone,
        sender: &TypedSocketSender<MessageFromDrone>,
    ) {
        match message {
            MessageToDrone::PushNetworkPolicy(wire) => {
                let action_id = wire.action_id.clone();
                self.idempotent(&action_id, sender, || async {
                    self.egress_publisher
                        .push_network_policy(&wire.backend_id, &wire.policy, wire.version)
                        .await
                })
                .await;
            }
            MessageToDrone::PushCredential(wire) => {
                let action_id = wire.action_id.clone();
                self.idempotent(&action_id, sender, || async {
                    self.egress_publisher
                        .push_credential(&wire.backend_id, &wire.binding)
                        .await
                })
                .await;
            }
            MessageToDrone::RevokeCredential {
                action_id,
                binding_id,
            } => {
                self.idempotent(&action_id, sender, || async {
                    self.egress_publisher.revoke_credential(&binding_id).await
                })
                .await;
            }
            MessageToDrone::Snapshot(wire) => {
                let action_id = wire.action_id.clone();
                self.idempotent(&action_id, sender, || async {
                    self.snapshot_runtime
                        .snapshot(&wire.backend_id, wire.ttl_days, wire.name.as_deref())
                        .await
                        .map(|_result| ())
                })
                .await;
            }
            _ => {}
        }
    }

    async fn idempotent<F, Fut>(
        &self,
        action_id: &str,
        sender: &TypedSocketSender<MessageFromDrone>,
        operation: F,
    ) where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<(), anyhow::Error>>,
    {
        if self.already_processed(action_id) {
            tracing::debug!(action_id = %action_id, "Duplicate extension action; re-acking");
            self.ack(sender, action_id);
            return;
        }

        match operation().await {
            Ok(()) => {
                tracing::info!(action_id = %action_id, "Extension action succeeded");
                self.ack(sender, action_id);
            }
            Err(err) => {
                tracing::error!(?err, action_id = %action_id, "Extension action failed");
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
    use plane_common::protocol::{PushCredentialWire, PushNetworkPolicyWire, SnapshotCommandWire};
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
            _binding: &CredentialPayloadWire,
        ) -> Result<(), anyhow::Error> {
            self.credential_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn revoke_credential(&self, _binding_id: &str) -> Result<(), anyhow::Error> {
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

        let msg = MessageToDrone::PushNetworkPolicy(PushNetworkPolicyWire {
            action_id: "xak-p1".to_string(),
            backend_id: "bk-test".to_string(),
            policy: serde_json::json!({"mode": "deny-all"}),
            version: 2,
        });

        handler.dispatch(msg, &sender).await;

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

        let msg = MessageToDrone::PushCredential(PushCredentialWire {
            action_id: "xak-c1".to_string(),
            backend_id: "bk-test".to_string(),
            binding: CredentialPayloadWire {
                id: "cred-001".to_string(),
                destination: "api.openai.com".to_string(),
                header_name: "Authorization".to_string(),
                header_format: "Bearer {value}".to_string(),
                value: "sk-test123".to_string(),
            },
        });

        handler.dispatch(msg, &sender).await;

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

        let msg = MessageToDrone::RevokeCredential {
            action_id: "xak-r1".to_string(),
            binding_id: "cred-001".to_string(),
        };

        handler.dispatch(msg, &sender).await;

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

        let msg = MessageToDrone::Snapshot(SnapshotCommandWire {
            action_id: "xak-s1".to_string(),
            backend_id: "bk-test".to_string(),
            ttl_days: Some(30),
            name: Some("pre-deploy".to_string()),
        });

        handler.dispatch(msg, &sender).await;

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

        let msg = MessageToDrone::PushNetworkPolicy(PushNetworkPolicyWire {
            action_id: "xak-dup".to_string(),
            backend_id: "bk-test".to_string(),
            policy: serde_json::json!({"mode": "allow-all"}),
            version: 1,
        });

        handler.dispatch(msg.clone(), &sender).await;
        handler.dispatch(msg, &sender).await;

        assert_eq!(publisher.policy_calls.load(Ordering::SeqCst), 1);
        let msgs = messages.lock().expect("lock");
        assert_eq!(msgs.len(), 2);
    }

    #[tokio::test]
    async fn clear_processed_allows_reprocessing() {
        let (publisher, _snapshot_rt, handler) = make_handler();
        let (sender, _messages) = make_test_sender();

        let msg = MessageToDrone::PushNetworkPolicy(PushNetworkPolicyWire {
            action_id: "xak-clear".to_string(),
            backend_id: "bk-test".to_string(),
            policy: serde_json::json!({"mode": "deny-all"}),
            version: 1,
        });

        handler.dispatch(msg.clone(), &sender).await;
        assert_eq!(publisher.policy_calls.load(Ordering::SeqCst), 1);

        handler.clear_processed();
        handler.dispatch(msg, &sender).await;
        assert_eq!(publisher.policy_calls.load(Ordering::SeqCst), 2);
    }
}
