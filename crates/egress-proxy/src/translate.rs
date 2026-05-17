use common::ext::network::{NetworkPolicy, NetworkPolicyMode};
use envoy_types::pb::envoy::config::{
    cluster::v3::Cluster, listener::v3::Listener, route::v3::RouteConfiguration,
};

pub struct XdsSnapshot {
    pub version: String,
    pub listeners: Vec<Listener>,
    pub routes: Vec<RouteConfiguration>,
    pub clusters: Vec<Cluster>,
}

pub fn translate_policy(sandbox_id: &str, policy: &NetworkPolicy, version: u64) -> XdsSnapshot {
    let version_str = version.to_string();

    match policy.mode {
        NetworkPolicyMode::DenyAll => translate_deny_all(sandbox_id, version_str),
        NetworkPolicyMode::AllowAll => translate_allow_all(sandbox_id, version_str),
        NetworkPolicyMode::Rules => translate_rules(sandbox_id, policy, version_str),
    }
}

fn translate_deny_all(sandbox_id: &str, version: String) -> XdsSnapshot {
    use envoy_types::pb::envoy::config::listener::v3::filter::ConfigType;
    use envoy_types::pb::envoy::config::listener::v3::{Filter, FilterChain};
    use envoy_types::pb::envoy::extensions::filters::network::tcp_proxy::v3::tcp_proxy::ClusterSpecifier;
    use envoy_types::pb::envoy::extensions::filters::network::tcp_proxy::v3::TcpProxy;
    use envoy_types::pb::google::protobuf::Any;
    use prost::Message;

    let tcp_proxy = TcpProxy {
        stat_prefix: format!("{sandbox_id}_deny"),
        cluster_specifier: Some(ClusterSpecifier::Cluster("blackhole".to_string())),
        ..Default::default()
    };

    let filter = Filter {
        name: "envoy.filters.network.tcp_proxy".to_string(),
        config_type: Some(ConfigType::TypedConfig(Any {
            type_url: "type.googleapis.com/envoy.extensions.filters.network.tcp_proxy.v3.TcpProxy"
                .to_string(),
            value: tcp_proxy.encode_to_vec(),
        })),
    };

    let listener = Listener {
        name: format!("{sandbox_id}_deny_all"),
        filter_chains: vec![FilterChain {
            filters: vec![filter],
            ..Default::default()
        }],
        ..Default::default()
    };

    XdsSnapshot {
        version,
        listeners: vec![listener],
        routes: vec![],
        clusters: vec![],
    }
}

fn translate_allow_all(sandbox_id: &str, version: String) -> XdsSnapshot {
    use envoy_types::pb::envoy::config::cluster::v3::cluster::{
        ClusterDiscoveryType, DiscoveryType,
    };

    let passthrough = Cluster {
        name: format!("{sandbox_id}_passthrough"),
        cluster_discovery_type: Some(ClusterDiscoveryType::Type(
            DiscoveryType::OriginalDst as i32,
        )),
        ..Default::default()
    };

    XdsSnapshot {
        version,
        listeners: vec![],
        routes: vec![],
        clusters: vec![passthrough],
    }
}

fn translate_rules(_sandbox_id: &str, _policy: &NetworkPolicy, version: String) -> XdsSnapshot {
    XdsSnapshot {
        version,
        listeners: vec![],
        routes: vec![],
        clusters: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_all_produces_black_hole_listener() {
        let policy = NetworkPolicy {
            mode: NetworkPolicyMode::DenyAll,
            rules: None,
        };

        let snapshot = translate_policy("sb-test-001", &policy, 1);

        assert_eq!(snapshot.version, "1");
        assert_eq!(
            snapshot.listeners.len(),
            1,
            "deny-all needs one listener to intercept traffic"
        );
        let listener = &snapshot.listeners[0];
        assert!(
            listener.name.contains("sb-test-001"),
            "listener name should contain sandbox_id"
        );
    }

    #[test]
    fn allow_all_produces_passthrough_cluster() {
        let policy = NetworkPolicy {
            mode: NetworkPolicyMode::AllowAll,
            rules: None,
        };

        let snapshot = translate_policy("sb-test-002", &policy, 1);

        assert_eq!(
            snapshot.clusters.len(),
            1,
            "allow-all needs a passthrough cluster"
        );
        let cluster = &snapshot.clusters[0];
        assert!(
            cluster.name.contains("passthrough"),
            "cluster should be named passthrough"
        );
    }

    #[test]
    fn deny_all_has_no_routes() {
        let policy = NetworkPolicy {
            mode: NetworkPolicyMode::DenyAll,
            rules: None,
        };

        let snapshot = translate_policy("sb-test-003", &policy, 1);

        assert!(snapshot.routes.is_empty(), "deny-all should have no routes");
    }
}
