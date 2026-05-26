//! Egress proxy setup for the sandbox container.
//!
//! Runs once at container bootstrap (via the `sandbox-init` binary, before
//! `agentd` itself starts) so that all subsequent traffic from the sandbox
//! workload is routed through the egress proxy with the proxy's CA trusted.
//!
//! Environment contract (set by the drone when egress is configured):
//! - `SANDBOX_EGRESS_PROXY` — `host:port` form of the HTTP proxy. Used for
//!   user-facing env vars like `HTTP_PROXY` so application code that respects
//!   them gets the hostname.
//! - `SANDBOX_EGRESS_PROXY_IP` — pre-resolved IPv4 address of the proxy host.
//!   Used for the `iptables` DNAT rules so the in-container DNS bootstrap is
//!   skipped.
//! - `SANDBOX_EGRESS_CA_PEM` — PEM-encoded CA certificate the proxy presents
//!   for HTTPS interception. Written to disk and appended to the system bundle.
//!
//! All three must be present together. If `SANDBOX_EGRESS_PROXY` is absent,
//! egress setup is a no-op (the sandbox runs without interception, suitable
//! for trusted runtimes or dev usage).

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

const CA_FILE_PATH: &str = "/etc/ssl/certs/sandbox-egress-ca.crt";
const SYSTEM_BUNDLE_PATH: &str = "/etc/ssl/certs/ca-certificates.crt";

#[derive(Debug, thiserror::Error)]
pub enum EgressError {
    #[error("environment variable {0} is set but {1} is missing; all three of SANDBOX_EGRESS_PROXY, SANDBOX_EGRESS_PROXY_IP, SANDBOX_EGRESS_CA_PEM must be provided together")]
    PartialEnv(&'static str, &'static str),
    #[error("SANDBOX_EGRESS_PROXY value {0:?} is not in host:port form")]
    BadProxy(String),
    #[error("SANDBOX_EGRESS_PROXY port {0:?} is not a valid u16")]
    BadPort(String),
    #[error("iptables command failed: {0}")]
    Iptables(String),
    #[error("CA cert IO: {0}")]
    Io(#[from] std::io::Error),
}

/// Parsed egress configuration. `None` means egress is not configured for this
/// sandbox; that's a valid state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `host:port` form, suitable for `HTTP_PROXY` and friends.
    pub proxy_addr: String,
    /// Pre-resolved proxy IPv4 address, for iptables rules.
    pub proxy_ip: String,
    /// HTTP port (cleartext intercept target).
    pub proxy_port: u16,
    /// HTTPS port — convention: proxy_port + 1.
    pub https_port: u16,
    /// PEM-encoded CA bundle the proxy presents for HTTPS interception.
    pub ca_pem: String,
}

impl Config {
    /// Read the egress configuration from process environment.
    ///
    /// Returns `Ok(None)` when egress is not configured (no `SANDBOX_EGRESS_PROXY`).
    /// Returns `Ok(Some(_))` when all three required variables are present and parse.
    /// Returns `Err(_)` when `SANDBOX_EGRESS_PROXY` is set but the contract is
    /// otherwise broken — that's a misconfigured drone and we fail loudly.
    pub fn from_env() -> Result<Option<Self>, EgressError> {
        let proxy_addr = match std::env::var("SANDBOX_EGRESS_PROXY") {
            Ok(v) if !v.is_empty() => v,
            _ => return Ok(None),
        };

        let proxy_ip = std::env::var("SANDBOX_EGRESS_PROXY_IP")
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or(EgressError::PartialEnv(
                "SANDBOX_EGRESS_PROXY",
                "SANDBOX_EGRESS_PROXY_IP",
            ))?;

        let ca_pem = std::env::var("SANDBOX_EGRESS_CA_PEM")
            .ok()
            .filter(|v| !v.is_empty())
            .ok_or(EgressError::PartialEnv(
                "SANDBOX_EGRESS_PROXY",
                "SANDBOX_EGRESS_CA_PEM",
            ))?;

        let (_, port_str) = proxy_addr
            .rsplit_once(':')
            .ok_or_else(|| EgressError::BadProxy(proxy_addr.clone()))?;
        let proxy_port: u16 = port_str
            .parse()
            .map_err(|_| EgressError::BadPort(port_str.to_string()))?;
        let https_port = proxy_port.checked_add(1).ok_or(EgressError::BadPort(
            "https port would overflow u16".to_string(),
        ))?;

        Ok(Some(Config {
            proxy_addr,
            proxy_ip,
            proxy_port,
            https_port,
            ca_pem,
        }))
    }

    /// Iptables rules this config installs. Exposed for testability — `install`
    /// invokes the binary directly.
    pub fn iptables_rules(&self) -> Vec<Vec<String>> {
        vec![
            vec![
                "-t".into(),
                "nat".into(),
                "-A".into(),
                "OUTPUT".into(),
                "-p".into(),
                "tcp".into(),
                "--dport".into(),
                "80".into(),
                "-j".into(),
                "DNAT".into(),
                "--to-destination".into(),
                format!("{}:{}", self.proxy_ip, self.proxy_port),
            ],
            vec![
                "-t".into(),
                "nat".into(),
                "-A".into(),
                "OUTPUT".into(),
                "-p".into(),
                "tcp".into(),
                "--dport".into(),
                "443".into(),
                "-j".into(),
                "DNAT".into(),
                "--to-destination".into(),
                format!("{}:{}", self.proxy_ip, self.https_port),
            ],
        ]
    }

    /// Set the env vars that downstream user processes use to pick up the
    /// proxy's CA. Modifies the current process env so any exec'd child
    /// inherits them.
    ///
    /// Deliberately does NOT set `HTTP_PROXY` / `HTTPS_PROXY`. The egress
    /// proxy's `host:port` (`SANDBOX_EGRESS_PROXY`) points at the proxy's
    /// transparent listener — not its explicit forward-proxy port. Iptables
    /// DNAT is the contract; a proxy-aware client that sees `HTTP_PROXY`
    /// would send `GET http://origin/...` to the transparent port, which
    /// expects DNAT'd connections and would reject the explicit form.
    pub fn export_env_for_children(&self, ca_path: &Path) {
        // sandbox-init runs strictly before exec'ing the workload — single-threaded.
        std::env::set_var("NODE_EXTRA_CA_CERTS", ca_path);
        std::env::set_var("REQUESTS_CA_BUNDLE", SYSTEM_BUNDLE_PATH);
        std::env::set_var("SSL_CERT_FILE", SYSTEM_BUNDLE_PATH);
    }

    /// Install the configuration: write the CA cert, append to the system
    /// bundle, install iptables NAT rules, export environment for children.
    /// Designed for `sandbox-init` to call before exec'ing the workload.
    pub fn install(&self) -> Result<(), EgressError> {
        let ca_path = PathBuf::from(CA_FILE_PATH);
        write_ca(&ca_path, &self.ca_pem)?;
        append_to_system_bundle(&self.ca_pem)?;
        for rule in self.iptables_rules() {
            run_iptables(&rule)?;
        }
        self.export_env_for_children(&ca_path);
        Ok(())
    }
}

fn write_ca(path: &Path, pem: &str) -> Result<(), EgressError> {
    fs::write(path, pem)?;
    Ok(())
}

fn append_to_system_bundle(pem: &str) -> Result<(), EgressError> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(SYSTEM_BUNDLE_PATH)?;
    // Always emit a separator newline. The existing bundle may or may not end
    // in '\n' — we can't easily check via append-only, and a stray blank line
    // is harmless to every PEM parser. The earlier `pem.ends_with('\n')` check
    // was on the wrong end and would skip the separator when the bundle lacked
    // a trailing newline, fusing the new BEGIN line onto the last cert.
    file.write_all(b"\n")?;
    file.write_all(pem.as_bytes())?;
    if !pem.ends_with('\n') {
        file.write_all(b"\n")?;
    }
    Ok(())
}

fn run_iptables(args: &[String]) -> Result<(), EgressError> {
    let output = Command::new("iptables")
        .args(args)
        .output()
        .map_err(|e| EgressError::Iptables(format!("failed to spawn iptables: {e}")))?;
    if !output.status.success() {
        return Err(EgressError::Iptables(format!(
            "iptables {} exited {}: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr),
        )));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Env-touching tests serialize via this lock so they don't race.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn clear_env() {
        for k in [
            "SANDBOX_EGRESS_PROXY",
            "SANDBOX_EGRESS_PROXY_IP",
            "SANDBOX_EGRESS_CA_PEM",
        ] {
            std::env::remove_var(k);
        }
    }

    #[test]
    fn from_env_returns_none_when_proxy_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        assert_eq!(Config::from_env().unwrap(), None);
    }

    #[test]
    fn from_env_returns_none_when_proxy_empty() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("SANDBOX_EGRESS_PROXY", "");
        assert_eq!(Config::from_env().unwrap(), None);
    }

    #[test]
    fn from_env_errors_when_ip_missing() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("SANDBOX_EGRESS_PROXY", "egress.local:3128");
        std::env::set_var(
            "SANDBOX_EGRESS_CA_PEM",
            "-----BEGIN CERTIFICATE-----\n-----END CERTIFICATE-----\n",
        );
        let err = Config::from_env().unwrap_err();
        assert!(matches!(
            err,
            EgressError::PartialEnv(_, "SANDBOX_EGRESS_PROXY_IP")
        ));
    }

    #[test]
    fn from_env_errors_when_ca_missing() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("SANDBOX_EGRESS_PROXY", "egress.local:3128");
        std::env::set_var("SANDBOX_EGRESS_PROXY_IP", "10.0.0.5");
        let err = Config::from_env().unwrap_err();
        assert!(matches!(
            err,
            EgressError::PartialEnv(_, "SANDBOX_EGRESS_CA_PEM")
        ));
    }

    #[test]
    fn from_env_parses_full_config() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("SANDBOX_EGRESS_PROXY", "egress.local:3128");
        std::env::set_var("SANDBOX_EGRESS_PROXY_IP", "10.0.0.5");
        std::env::set_var(
            "SANDBOX_EGRESS_CA_PEM",
            "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n",
        );
        let cfg = Config::from_env().unwrap().unwrap();
        assert_eq!(cfg.proxy_addr, "egress.local:3128");
        assert_eq!(cfg.proxy_ip, "10.0.0.5");
        assert_eq!(cfg.proxy_port, 3128);
        assert_eq!(cfg.https_port, 3129);
        assert!(cfg.ca_pem.starts_with("-----BEGIN CERTIFICATE-----"));
    }

    #[test]
    fn from_env_rejects_proxy_without_port() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("SANDBOX_EGRESS_PROXY", "egress.local");
        std::env::set_var("SANDBOX_EGRESS_PROXY_IP", "10.0.0.5");
        std::env::set_var("SANDBOX_EGRESS_CA_PEM", "x");
        assert!(matches!(Config::from_env(), Err(EgressError::BadProxy(_))));
    }

    #[test]
    fn iptables_rules_target_proxy_ip_and_correct_ports() {
        let cfg = Config {
            proxy_addr: "egress.local:3128".into(),
            proxy_ip: "10.0.0.5".into(),
            proxy_port: 3128,
            https_port: 3129,
            ca_pem: "x".into(),
        };
        let rules = cfg.iptables_rules();
        assert_eq!(rules.len(), 2);

        let http_rule = &rules[0];
        assert!(http_rule.contains(&"80".to_string()));
        assert!(http_rule.contains(&"10.0.0.5:3128".to_string()));

        let https_rule = &rules[1];
        assert!(https_rule.contains(&"443".to_string()));
        assert!(https_rule.contains(&"10.0.0.5:3129".to_string()));
    }
}
