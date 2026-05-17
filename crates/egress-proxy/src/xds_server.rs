use crate::snapshot_store::SnapshotStore;
use envoy_types::pb::envoy::service::discovery::v3::aggregated_discovery_service_server::AggregatedDiscoveryService;
use envoy_types::pb::envoy::service::discovery::v3::{
    DeltaDiscoveryRequest, DeltaDiscoveryResponse, DiscoveryRequest, DiscoveryResponse,
};
use envoy_types::pb::google::protobuf::Any;
use prost::Message;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

const TYPE_URL_LDS: &str = "type.googleapis.com/envoy.config.listener.v3.Listener";
const TYPE_URL_RDS: &str = "type.googleapis.com/envoy.config.route.v3.RouteConfiguration";
const TYPE_URL_CDS: &str = "type.googleapis.com/envoy.config.cluster.v3.Cluster";

pub struct XdsServer {
    store: SnapshotStore,
}

impl XdsServer {
    pub fn new(store: SnapshotStore) -> Self {
        Self { store }
    }

    fn build_response(&self, type_url: &str) -> DiscoveryResponse {
        let resources = self.collect_resources(type_url);
        DiscoveryResponse {
            version_info: "1".to_string(),
            resources,
            type_url: type_url.to_string(),
            nonce: uuid::Uuid::new_v4().to_string(),
            ..Default::default()
        }
    }

    fn collect_resources(&self, type_url: &str) -> Vec<Any> {
        let mut resources = Vec::new();

        for snapshot in self.store.all_snapshots() {
            match type_url {
                TYPE_URL_LDS => {
                    for listener in &snapshot.listeners {
                        resources.push(Any {
                            type_url: TYPE_URL_LDS.to_string(),
                            value: listener.encode_to_vec(),
                        });
                    }
                }
                TYPE_URL_RDS => {
                    for route in &snapshot.routes {
                        resources.push(Any {
                            type_url: TYPE_URL_RDS.to_string(),
                            value: route.encode_to_vec(),
                        });
                    }
                }
                TYPE_URL_CDS => {
                    for cluster in &snapshot.clusters {
                        resources.push(Any {
                            type_url: TYPE_URL_CDS.to_string(),
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

#[tonic::async_trait]
impl AggregatedDiscoveryService for XdsServer {
    type StreamAggregatedResourcesStream =
        Pin<Box<ReceiverStream<Result<DiscoveryResponse, Status>>>>;

    async fn stream_aggregated_resources(
        &self,
        request: Request<Streaming<DiscoveryRequest>>,
    ) -> Result<Response<Self::StreamAggregatedResourcesStream>, Status> {
        let mut stream = request.into_inner();
        let store = self.store.clone();
        let (tx, rx) = mpsc::channel(32);

        tokio::spawn(async move {
            let server = XdsServer { store };
            while let Ok(Some(req)) = stream.message().await {
                let response = server.build_response(&req.type_url);
                if tx.send(Ok(response)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    type DeltaAggregatedResourcesStream =
        Pin<Box<ReceiverStream<Result<DeltaDiscoveryResponse, Status>>>>;

    async fn delta_aggregated_resources(
        &self,
        _request: Request<Streaming<DeltaDiscoveryRequest>>,
    ) -> Result<Response<Self::DeltaAggregatedResourcesStream>, Status> {
        Err(Status::unimplemented("Delta xDS not supported"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translate::translate_policy;
    use common::ext::network::{NetworkPolicy, NetworkPolicyMode};

    #[test]
    fn collect_listeners_from_deny_all() {
        let store = SnapshotStore::new();
        let snapshot = translate_policy(
            "sb-xds-001",
            &NetworkPolicy {
                mode: NetworkPolicyMode::DenyAll,
                rules: None,
            },
            1,
        );
        store.update("sb-xds-001", snapshot);

        let server = XdsServer::new(store);
        let resources = server.collect_resources(TYPE_URL_LDS);

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].type_url, TYPE_URL_LDS);
    }

    #[test]
    fn collect_clusters_from_allow_all() {
        let store = SnapshotStore::new();
        let snapshot = translate_policy(
            "sb-xds-002",
            &NetworkPolicy {
                mode: NetworkPolicyMode::AllowAll,
                rules: None,
            },
            1,
        );
        store.update("sb-xds-002", snapshot);

        let server = XdsServer::new(store);
        let resources = server.collect_resources(TYPE_URL_CDS);

        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].type_url, TYPE_URL_CDS);
    }

    #[test]
    fn collect_merges_multiple_sandboxes() {
        let store = SnapshotStore::new();

        store.update(
            "sb-a",
            translate_policy(
                "sb-a",
                &NetworkPolicy {
                    mode: NetworkPolicyMode::DenyAll,
                    rules: None,
                },
                1,
            ),
        );
        store.update(
            "sb-b",
            translate_policy(
                "sb-b",
                &NetworkPolicy {
                    mode: NetworkPolicyMode::DenyAll,
                    rules: None,
                },
                1,
            ),
        );

        let server = XdsServer::new(store);
        let resources = server.collect_resources(TYPE_URL_LDS);

        assert_eq!(
            resources.len(),
            2,
            "should merge listeners from both sandboxes"
        );
    }

    #[test]
    fn collect_unknown_type_returns_empty() {
        let store = SnapshotStore::new();
        store.update(
            "sb-c",
            translate_policy(
                "sb-c",
                &NetworkPolicy {
                    mode: NetworkPolicyMode::DenyAll,
                    rules: None,
                },
                1,
            ),
        );

        let server = XdsServer::new(store);
        let resources = server.collect_resources("type.googleapis.com/unknown");

        assert!(resources.is_empty());
    }
}
