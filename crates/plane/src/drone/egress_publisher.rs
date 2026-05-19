use super::extension_handler::{EgressPublisher, SnapshotResult, SnapshotRuntime};
use plane_common::protocol::CredentialPayloadWire;
use url::Url;

pub struct HttpEgressPublisher {
    client: reqwest::Client,
    admin_url: Url,
}

impl HttpEgressPublisher {
    pub fn new(proxy_admin_url: Url) -> Self {
        Self {
            client: reqwest::Client::new(),
            admin_url: proxy_admin_url,
        }
    }

    async fn send_checked(&self, req: reqwest::RequestBuilder) -> Result<(), anyhow::Error> {
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Egress proxy returned {status}: {text}"));
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl EgressPublisher for HttpEgressPublisher {
    async fn push_network_policy(
        &self,
        _backend_id: &str,
        _policy: &serde_json::Value,
        _version: u64,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }

    async fn push_credential(
        &self,
        _backend_id: &str,
        binding: &CredentialPayloadWire,
    ) -> Result<(), anyhow::Error> {
        let url = self
            .admin_url
            .join("/admin/credentials")
            .map_err(|e| anyhow::anyhow!("Invalid admin URL: {e}"))?;

        self.send_checked(self.client.post(url.as_str()).json(binding))
            .await?;

        tracing::info!(
            destination = binding.destination.as_str(),
            header_name = binding.header_name.as_str(),
            "Credential pushed to egress proxy"
        );
        Ok(())
    }

    async fn revoke_credential(&self, binding_id: &str) -> Result<(), anyhow::Error> {
        let path = format!("/admin/credentials/{binding_id}");
        let url = self
            .admin_url
            .join(&path)
            .map_err(|e| anyhow::anyhow!("Invalid admin URL: {e}"))?;

        self.send_checked(self.client.delete(url.as_str())).await?;

        tracing::info!(binding_id, "Credential revoked from egress proxy");
        Ok(())
    }
}

pub struct NoopSnapshotRuntime;

#[async_trait::async_trait]
impl SnapshotRuntime for NoopSnapshotRuntime {
    async fn snapshot(
        &self,
        _backend_id: &str,
        _ttl_days: Option<u32>,
        _name: Option<&str>,
    ) -> Result<SnapshotResult, anyhow::Error> {
        Err(anyhow::anyhow!("Snapshot runtime not configured"))
    }
}
