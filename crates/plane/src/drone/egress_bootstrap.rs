//! One-time, drone-startup egress proxy bootstrap (with periodic refresh).
//!
//! Fetches the proxy's CA certificate from its admin endpoint and resolves the
//! proxy hostname to an IPv4 once, so each sandbox spawn can inject both via
//! env vars instead of paying for a DNS bootstrap + HTTP fetch inside the
//! fresh container's network namespace.

use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tokio::net::lookup_host;
use url::Url;

use super::runtime::docker::EgressContext;
use crate::util::GuardHandle;

const ADMIN_CA_PATH: &str = "admin/ca.crt";
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
const REFRESH_INTERVAL: Duration = Duration::from_secs(3600);

/// Handle to the live egress context held by the drone. The inner value is
/// swapped in by a background refresh task; readers (the spawn path) always
/// see a consistent snapshot via `load()`.
#[derive(Clone)]
pub struct EgressHandle {
    inner: Arc<RwLock<Arc<EgressContext>>>,
    _refresh_handle: Arc<GuardHandle>,
}

impl EgressHandle {
    pub fn load(&self) -> Arc<EgressContext> {
        self.inner
            .read()
            .expect("egress context lock poisoned")
            .clone()
    }
}

/// Bootstrap an `EgressHandle` from the drone's egress-proxy URLs.
///
/// - `admin_url` is the egress-proxy admin HTTP base (e.g.
///   `http://egress-proxy:3129/`) used to download the CA the proxy presents.
///   Must end in `/` so the CA path appends correctly.
/// - `proxy_addr` is the `host:port` of the proxy reachable from sandbox
///   containers (e.g. `host.docker.internal:3128`), surfaced into sandboxes as
///   `SANDBOX_EGRESS_PROXY`.
///
/// Fails loudly when the initial fetch or DNS resolve doesn't succeed — the
/// operator wants to know at drone startup, not silently when sandboxes hit
/// the unenforced state. After bootstrap, a background task refreshes both
/// the CA and the resolved IP every `REFRESH_INTERVAL` so a proxy restart
/// (IP reassignment) or CA rotation propagates without a drone restart.
pub async fn bootstrap(admin_url: &Url, proxy_addr: &str) -> Result<EgressHandle> {
    let admin_url = ensure_trailing_slash(admin_url);
    let initial = fetch_context(&admin_url, proxy_addr).await?;
    tracing::info!(
        proxy_addr = %initial.addr,
        proxy_ip = %initial.ip,
        ca_bytes = initial.ca_pem.len(),
        "Egress bootstrap complete",
    );

    let inner = Arc::new(RwLock::new(Arc::new(initial)));

    let refresh_handle = {
        let inner = inner.clone();
        let admin_url = admin_url.clone();
        let proxy_addr = proxy_addr.to_string();
        GuardHandle::new(async move {
            refresh_loop(inner, admin_url, proxy_addr).await;
        })
    };

    Ok(EgressHandle {
        inner,
        _refresh_handle: Arc::new(refresh_handle),
    })
}

/// Build a fresh `EgressContext` by talking to the admin endpoint and
/// resolving DNS. Used both at bootstrap and from the refresh task.
async fn fetch_context(admin_url: &Url, proxy_addr: &str) -> Result<EgressContext> {
    let host = proxy_addr
        .rsplit_once(':')
        .map(|(host, _)| host)
        .ok_or_else(|| anyhow!("egress-proxy-addr {:?} must be host:port", proxy_addr))?;

    let ip = resolve_ipv4(host)
        .await
        .with_context(|| format!("resolving egress proxy host {host:?}"))?;

    let ca_url = admin_url
        .join(ADMIN_CA_PATH)
        .with_context(|| format!("building CA URL from {admin_url}"))?;

    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .context("building CA fetch HTTP client")?;

    let response = client
        .get(ca_url.clone())
        .send()
        .await
        .with_context(|| format!("fetching CA from {ca_url}"))?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "egress CA fetch from {ca_url} failed with HTTP {}",
            response.status()
        ));
    }
    let ca_pem = response
        .text()
        .await
        .with_context(|| format!("reading CA body from {ca_url}"))?;
    if !ca_pem.contains("BEGIN CERTIFICATE") {
        return Err(anyhow!(
            "egress CA fetched from {ca_url} does not look like a PEM certificate"
        ));
    }

    Ok(EgressContext {
        addr: proxy_addr.to_string(),
        ip,
        ca_pem: Arc::new(ca_pem),
    })
}

async fn refresh_loop(inner: Arc<RwLock<Arc<EgressContext>>>, admin_url: Url, proxy_addr: String) {
    loop {
        tokio::time::sleep(REFRESH_INTERVAL).await;
        match fetch_context(&admin_url, &proxy_addr).await {
            Ok(next) => {
                let (ip_changed, ca_changed, prev_ip, next_ip) = {
                    let mut guard = inner.write().expect("egress context lock poisoned");
                    let ip_changed = guard.ip != next.ip;
                    let ca_changed = guard.ca_pem != next.ca_pem;
                    let prev_ip = guard.ip.clone();
                    let next_ip = next.ip.clone();
                    *guard = Arc::new(next);
                    (ip_changed, ca_changed, prev_ip, next_ip)
                };
                if ip_changed || ca_changed {
                    // Loud: the snapshot moved, but every sandbox already
                    // running on this drone has its iptables rules + appended
                    // CA frozen from sandbox-init time. New spawns will pick
                    // up the new values; existing sandboxes will keep routing
                    // to the previous IP / trusting the previous CA. Operators
                    // need to know so they can restart affected workloads.
                    tracing::warn!(
                        ip_changed,
                        ca_changed,
                        previous_ip = %prev_ip,
                        new_ip = %next_ip,
                        "Egress refresh changed IP or CA — already-running sandboxes still hold the previous values and need to be restarted to pick up the change",
                    );
                }
            }
            Err(e) => {
                tracing::warn!(error = ?e, "Egress refresh failed; keeping previous values");
            }
        }
    }
}

fn ensure_trailing_slash(url: &Url) -> Url {
    if url.path().ends_with('/') {
        url.clone()
    } else {
        let mut u = url.clone();
        u.set_path(&format!("{}/", u.path()));
        u
    }
}

async fn resolve_ipv4(host: &str) -> Result<String> {
    // tokio::net::lookup_host runs on the runtime's blocking pool internally,
    // so the calling task never blocks a worker on getaddrinfo.
    let mut addrs = lookup_host((host, 0u16))
        .await
        .with_context(|| format!("looking up {host}"))?;
    addrs
        .find_map(|addr| match addr.ip() {
            std::net::IpAddr::V4(v4) => Some(v4.to_string()),
            std::net::IpAddr::V6(_) => None,
        })
        .ok_or_else(|| anyhow!("no IPv4 address found for {host}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn ensure_trailing_slash_appends_when_missing() {
        let u: Url = "http://egress:3129".parse().unwrap();
        let out = ensure_trailing_slash(&u);
        assert_eq!(out.as_str(), "http://egress:3129/");
    }

    #[test]
    fn ensure_trailing_slash_preserves_existing_slash() {
        let u: Url = "http://egress:3129/v1/".parse().unwrap();
        let out = ensure_trailing_slash(&u);
        assert_eq!(out.as_str(), "http://egress:3129/v1/");
    }

    #[test]
    fn ensure_trailing_slash_appends_to_path_prefix() {
        // The bug: `http://egress:3129/v1`.join("admin/ca.crt")
        // resolves to `http://egress:3129/admin/ca.crt`, dropping "v1".
        // Forcing a trailing slash fixes it.
        let u: Url = "http://egress:3129/v1".parse().unwrap();
        let out = ensure_trailing_slash(&u);
        assert_eq!(out.as_str(), "http://egress:3129/v1/");
        let joined = out.join(ADMIN_CA_PATH).unwrap();
        assert_eq!(joined.as_str(), "http://egress:3129/v1/admin/ca.crt");
    }
}
