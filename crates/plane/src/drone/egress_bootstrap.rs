//! One-time, drone-startup egress proxy bootstrap.
//!
//! Fetches the proxy's CA certificate from its admin endpoint and resolves the
//! proxy hostname to an IPv4 once, so each sandbox spawn can inject both via
//! env vars instead of paying for a DNS bootstrap + HTTP fetch inside the
//! fresh container's network namespace.

use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use url::Url;

use super::runtime::docker::EgressContext;

const ADMIN_CA_PATH: &str = "/admin/ca.crt";
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

/// Bootstrap an `EgressContext` from the drone's egress-proxy URLs.
///
/// - `admin_url` is the egress-proxy admin HTTP base (e.g. `http://egress-proxy:3129/`)
///   used to download the CA the proxy presents.
/// - `proxy_addr` is the `host:port` of the proxy reachable from sandbox
///   containers (e.g. `host.docker.internal:3128`), surfaced into sandboxes as
///   `SANDBOX_EGRESS_PROXY`.
///
/// Fails loudly when either step doesn't succeed — the operator wants to know
/// at drone startup, not silently when sandboxes hit the unenforced state.
pub async fn bootstrap(admin_url: &Url, proxy_addr: &str) -> Result<EgressContext> {
    let host = proxy_addr
        .rsplit_once(':')
        .map(|(host, _)| host)
        .ok_or_else(|| anyhow!("egress-proxy-addr {:?} must be host:port", proxy_addr))?;

    let ip = resolve_ipv4(host).with_context(|| format!("resolving egress proxy host {host:?}"))?;

    let ca_url = admin_url
        .join(ADMIN_CA_PATH.trim_start_matches('/'))
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

fn resolve_ipv4(host: &str) -> Result<String> {
    // Append a throwaway port so we can use the std resolver; we only want the address.
    let mut addrs = (host, 0u16)
        .to_socket_addrs()
        .with_context(|| format!("looking up {host}"))?;
    addrs
        .find_map(|addr| match addr.ip() {
            std::net::IpAddr::V4(v4) => Some(v4.to_string()),
            std::net::IpAddr::V6(_) => None,
        })
        .ok_or_else(|| anyhow!("no IPv4 address found for {host}"))
}
