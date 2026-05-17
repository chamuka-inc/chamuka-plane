use common::ext::network::{NetworkPolicy, NetworkPolicyMode};
use envoy_types::pb::envoy::config::{
    cluster::v3::Cluster, listener::v3::Listener, route::v3::RouteConfiguration,
};

#[derive(Clone)]
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

fn translate_rules(sandbox_id: &str, policy: &NetworkPolicy, version: String) -> XdsSnapshot {
    use common::ext::network::RuleAction;
    use envoy_types::pb::envoy::config::cluster::v3::cluster::{
        ClusterDiscoveryType, DiscoveryType,
    };
    use envoy_types::pb::envoy::config::listener::v3::filter::ConfigType;
    use envoy_types::pb::envoy::config::listener::v3::{Filter, FilterChain};
    use envoy_types::pb::envoy::config::route::v3::{
        route::Action, route_action::ClusterSpecifier, Route, RouteAction, RouteMatch, VirtualHost,
    };
    use envoy_types::pb::envoy::extensions::filters::network::http_connection_manager::v3::{
        http_connection_manager::RouteSpecifier, HttpConnectionManager,
    };
    use envoy_types::pb::google::protobuf::Any;
    use prost::Message;

    let route_config_name = format!("{sandbox_id}_routes");
    let mut virtual_hosts = Vec::new();
    let mut clusters = Vec::new();

    let rules = policy.rules.as_deref().unwrap_or_default();

    for (idx, rule) in rules.iter().enumerate() {
        let action = rule.action.unwrap_or(RuleAction::Allow);
        let domains = rule
            .domains
            .as_deref()
            .unwrap_or(&["*".to_string()] as &[String])
            .to_vec();

        match action {
            RuleAction::Allow => {
                let cluster_name = format!("{sandbox_id}_rule_{idx}");

                let vhost = VirtualHost {
                    name: format!("{sandbox_id}_vhost_{idx}"),
                    domains: domains.clone(),
                    routes: vec![Route {
                        r#match: Some(RouteMatch {
                            path_specifier: Some(
                                envoy_types::pb::envoy::config::route::v3::route_match::PathSpecifier::Prefix(
                                    "/".to_string(),
                                ),
                            ),
                            ..Default::default()
                        }),
                        action: Some(Action::Route(RouteAction {
                            cluster_specifier: Some(ClusterSpecifier::Cluster(
                                cluster_name.clone(),
                            )),
                            ..Default::default()
                        })),
                        ..Default::default()
                    }],
                    ..Default::default()
                };
                virtual_hosts.push(vhost);

                let cluster = Cluster {
                    name: cluster_name,
                    cluster_discovery_type: Some(ClusterDiscoveryType::Type(
                        DiscoveryType::OriginalDst as i32,
                    )),
                    ..Default::default()
                };
                clusters.push(cluster);
            }
            RuleAction::Deny => {
                let vhost = VirtualHost {
                    name: format!("{sandbox_id}_vhost_{idx}_deny"),
                    domains,
                    routes: vec![Route {
                        r#match: Some(RouteMatch {
                            path_specifier: Some(
                                envoy_types::pb::envoy::config::route::v3::route_match::PathSpecifier::Prefix(
                                    "/".to_string(),
                                ),
                            ),
                            ..Default::default()
                        }),
                        action: Some(Action::DirectResponse(
                            envoy_types::pb::envoy::config::route::v3::DirectResponseAction {
                                status: 403,
                                ..Default::default()
                            },
                        )),
                        ..Default::default()
                    }],
                    ..Default::default()
                };
                virtual_hosts.push(vhost);
            }
        }
    }

    let route_configuration = RouteConfiguration {
        name: route_config_name.clone(),
        virtual_hosts,
        ..Default::default()
    };

    let hcm = HttpConnectionManager {
        stat_prefix: format!("{sandbox_id}_hcm"),
        route_specifier: Some(RouteSpecifier::Rds(
            envoy_types::pb::envoy::extensions::filters::network::http_connection_manager::v3::Rds {
                route_config_name,
                ..Default::default()
            },
        )),
        ..Default::default()
    };

    let filter = Filter {
        name: "envoy.filters.network.http_connection_manager".to_string(),
        config_type: Some(ConfigType::TypedConfig(Any {
            type_url: "type.googleapis.com/envoy.extensions.filters.network.http_connection_manager.v3.HttpConnectionManager".to_string(),
            value: hcm.encode_to_vec(),
        })),
    };

    let listener = Listener {
        name: format!("{sandbox_id}_http"),
        filter_chains: vec![FilterChain {
            filters: vec![filter],
            ..Default::default()
        }],
        ..Default::default()
    };

    XdsSnapshot {
        version,
        listeners: vec![listener],
        routes: vec![route_configuration],
        clusters,
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

    #[test]
    fn rules_mode_produces_listener_and_route() {
        use common::ext::network::{NetworkRule, RuleAction};

        let policy = NetworkPolicy {
            mode: NetworkPolicyMode::Rules,
            rules: Some(vec![NetworkRule {
                domains: Some(vec!["api.openai.com".to_string()]),
                cidrs: None,
                ports: Some(vec![443]),
                action: Some(RuleAction::Allow),
                inject: None,
            }]),
        };

        let snapshot = translate_policy("sb-test-004", &policy, 2);

        assert_eq!(snapshot.version, "2");
        assert!(
            !snapshot.listeners.is_empty(),
            "rules-mode needs at least one listener"
        );
        assert!(
            !snapshot.routes.is_empty(),
            "rules-mode needs route config for domain matching"
        );
        assert!(
            !snapshot.clusters.is_empty(),
            "rules-mode needs clusters for allowed destinations"
        );

        let route_config = &snapshot.routes[0];
        assert!(
            !route_config.virtual_hosts.is_empty(),
            "route config should have virtual hosts"
        );
        let vhost = &route_config.virtual_hosts[0];
        assert!(
            vhost.domains.contains(&"api.openai.com".to_string()),
            "virtual host should match the allowed domain"
        );
    }

    #[test]
    fn rules_mode_deny_rule_has_no_cluster() {
        use common::ext::network::{NetworkRule, RuleAction};

        let policy = NetworkPolicy {
            mode: NetworkPolicyMode::Rules,
            rules: Some(vec![NetworkRule {
                domains: Some(vec!["evil.com".to_string()]),
                cidrs: None,
                ports: None,
                action: Some(RuleAction::Deny),
                inject: None,
            }]),
        };

        let snapshot = translate_policy("sb-test-005", &policy, 1);

        assert!(
            snapshot.clusters.is_empty(),
            "deny-only rules should produce no clusters"
        );
    }
}
