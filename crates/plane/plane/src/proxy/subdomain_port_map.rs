use plane_common::protocol::{SubdomainPortRouteRemove, SubdomainPortRouteUpdate};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubdomainPortKey {
    pub sandbox_id_prefix: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct SubdomainPortEntry {
    pub address: SocketAddr,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub struct SubdomainPortMap {
    routes: Mutex<HashMap<SubdomainPortKey, SubdomainPortEntry>>,
}

impl Default for SubdomainPortMap {
    fn default() -> Self {
        Self::new()
    }
}

impl SubdomainPortMap {
    pub fn new() -> Self {
        Self {
            routes: Mutex::new(HashMap::new()),
        }
    }

    pub fn update(&self, msg: SubdomainPortRouteUpdate) {
        let key = SubdomainPortKey {
            sandbox_id_prefix: msg.sandbox_id_prefix,
            port: msg.port,
        };
        let addr = msg.address.0;
        let entry = SubdomainPortEntry {
            address: addr,
            expires_at: msg.expires_at,
        };
        self.routes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key, entry);
    }

    pub fn remove(&self, msg: SubdomainPortRouteRemove) {
        let key = SubdomainPortKey {
            sandbox_id_prefix: msg.sandbox_id_prefix,
            port: msg.port,
        };
        self.routes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key);
    }

    pub fn lookup(&self, sandbox_id_prefix: &str, port: u16) -> Option<SocketAddr> {
        let key = SubdomainPortKey {
            sandbox_id_prefix: sandbox_id_prefix.to_string(),
            port,
        };
        let routes = self.routes.lock().unwrap_or_else(|e| e.into_inner());
        let entry = routes.get(&key)?;
        if entry.expires_at < chrono::Utc::now() {
            return None;
        }
        Some(entry.address)
    }

    pub fn route_count(&self) -> usize {
        self.routes.lock().unwrap_or_else(|e| e.into_inner()).len()
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
