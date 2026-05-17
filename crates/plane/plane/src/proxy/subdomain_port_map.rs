use plane_common::protocol::{SubdomainPortRouteRemove, SubdomainPortRouteUpdate};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::RwLock;

#[derive(Debug, Clone)]
struct SubdomainPortEntry {
    address: SocketAddr,
    expires_at: chrono::DateTime<chrono::Utc>,
}

pub struct SubdomainPortMap {
    routes: RwLock<HashMap<String, HashMap<u16, SubdomainPortEntry>>>,
}

impl Default for SubdomainPortMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SubdomainPortMap {
    pub fn new() -> Self {
        Self {
            routes: RwLock::new(HashMap::new()),
        }
    }

    pub fn update(&self, msg: SubdomainPortRouteUpdate) {
        let entry = SubdomainPortEntry {
            address: msg.address.0,
            expires_at: msg.expires_at,
        };
        let mut routes = self.routes.write().expect("SubdomainPortMap lock poisoned");
        routes
            .entry(msg.sandbox_id_prefix)
            .or_default()
            .insert(msg.port, entry);

        routes.retain(|_, ports| {
            ports.retain(|_, e| e.expires_at >= chrono::Utc::now());
            !ports.is_empty()
        });
    }

    pub fn remove(&self, msg: SubdomainPortRouteRemove) {
        let mut routes = self.routes.write().expect("SubdomainPortMap lock poisoned");
        if let Some(ports) = routes.get_mut(&msg.sandbox_id_prefix) {
            ports.remove(&msg.port);
            if ports.is_empty() {
                routes.remove(&msg.sandbox_id_prefix);
            }
        }
    }

    pub fn lookup(&self, sandbox_id_prefix: &str, port: u16) -> Option<SocketAddr> {
        let routes = self.routes.read().expect("SubdomainPortMap lock poisoned");
        let entry = routes.get(sandbox_id_prefix)?.get(&port)?;
        if entry.expires_at < chrono::Utc::now() {
            return None;
        }
        Some(entry.address)
    }

    pub fn route_count(&self) -> usize {
        self.routes
            .read()
            .expect("SubdomainPortMap lock poisoned")
            .values()
            .map(|ports| ports.len())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use plane_common::log_types::BackendAddr;
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::time::Duration;

    fn future_time() -> chrono::DateTime<Utc> {
        Utc::now() + Duration::from_secs(3600)
    }

    fn past_time() -> chrono::DateTime<Utc> {
        Utc::now() - Duration::from_secs(3600)
    }

    #[test]
    fn update_and_lookup() {
        let map = SubdomainPortMap::new();
        let addr = SocketAddr::from_str("10.0.0.1:8080").unwrap();
        map.update(SubdomainPortRouteUpdate {
            sandbox_id_prefix: "sb-abc".to_string(),
            port: 3000,
            address: BackendAddr(addr),
            expires_at: future_time(),
        });

        let result = map.lookup("sb-abc", 3000);
        assert_eq!(result, Some(addr));
    }

    #[test]
    fn lookup_nonexistent_returns_none() {
        let map = SubdomainPortMap::new();
        assert_eq!(map.lookup("sb-nope", 3000), None);
    }

    #[test]
    fn remove_clears_entry() {
        let map = SubdomainPortMap::new();
        let addr = SocketAddr::from_str("10.0.0.2:9000").unwrap();
        map.update(SubdomainPortRouteUpdate {
            sandbox_id_prefix: "sb-xyz".to_string(),
            port: 8080,
            address: BackendAddr(addr),
            expires_at: future_time(),
        });
        assert_eq!(map.route_count(), 1);

        map.remove(SubdomainPortRouteRemove {
            sandbox_id_prefix: "sb-xyz".to_string(),
            port: 8080,
        });
        assert_eq!(map.route_count(), 0);
        assert_eq!(map.lookup("sb-xyz", 8080), None);
    }

    #[test]
    fn expired_entry_returns_none() {
        let map = SubdomainPortMap::new();
        let addr = SocketAddr::from_str("10.0.0.3:5000").unwrap();
        map.update(SubdomainPortRouteUpdate {
            sandbox_id_prefix: "sb-old".to_string(),
            port: 443,
            address: BackendAddr(addr),
            expires_at: past_time(),
        });

        assert_eq!(map.lookup("sb-old", 443), None);
    }

    #[test]
    fn expired_entries_evicted_on_update() {
        let map = SubdomainPortMap::new();
        let addr = SocketAddr::from_str("10.0.0.3:5000").unwrap();
        map.update(SubdomainPortRouteUpdate {
            sandbox_id_prefix: "sb-stale".to_string(),
            port: 443,
            address: BackendAddr(addr),
            expires_at: past_time(),
        });

        map.update(SubdomainPortRouteUpdate {
            sandbox_id_prefix: "sb-fresh".to_string(),
            port: 80,
            address: BackendAddr(addr),
            expires_at: future_time(),
        });

        assert_eq!(map.route_count(), 1, "stale entry should have been evicted");
    }

    #[test]
    fn different_ports_are_independent() {
        let map = SubdomainPortMap::new();
        let addr1 = SocketAddr::from_str("10.0.0.1:8080").unwrap();
        let addr2 = SocketAddr::from_str("10.0.0.1:9090").unwrap();

        map.update(SubdomainPortRouteUpdate {
            sandbox_id_prefix: "sb-multi".to_string(),
            port: 3000,
            address: BackendAddr(addr1),
            expires_at: future_time(),
        });
        map.update(SubdomainPortRouteUpdate {
            sandbox_id_prefix: "sb-multi".to_string(),
            port: 8080,
            address: BackendAddr(addr2),
            expires_at: future_time(),
        });

        assert_eq!(map.lookup("sb-multi", 3000), Some(addr1));
        assert_eq!(map.lookup("sb-multi", 8080), Some(addr2));
        assert_eq!(map.route_count(), 2);
    }
}
