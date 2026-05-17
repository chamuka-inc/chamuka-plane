use crate::error::CliError;
use common::types::ClusterState;
use serde::de::DeserializeOwned;

pub struct ControllerClient {
    http: reqwest::Client,
    base_url: String,
}

impl ControllerClient {
    pub fn new(mut base_url: String) -> Self {
        while base_url.ends_with('/') {
            base_url.pop();
        }
        Self {
            http: reqwest::Client::new(),
            base_url,
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, CliError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.http.get(&url).send().await?.error_for_status()?;
        let body = resp.json::<T>().await?;
        Ok(body)
    }

    async fn post_empty(&self, path: &str) -> Result<(), CliError> {
        let url = format!("{}{}", self.base_url, path);
        self.http.post(&url).send().await?.error_for_status()?;
        Ok(())
    }

    async fn delete_empty(&self, path: &str) -> Result<(), CliError> {
        let url = format!("{}{}", self.base_url, path);
        self.http.delete(&url).send().await?.error_for_status()?;
        Ok(())
    }

    pub async fn status(&self) -> Result<serde_json::Value, CliError> {
        self.get("/ctrl/status").await
    }

    pub async fn cluster_state(&self, cluster: &str) -> Result<ClusterState, CliError> {
        self.get(&format!("/ctrl/c/{cluster}/state")).await
    }

    pub async fn drain_drone(&self, cluster: &str, drone: &str) -> Result<(), CliError> {
        self.post_empty(&format!("/ctrl/c/{cluster}/d/{drone}/drain"))
            .await
    }

    pub async fn soft_terminate(&self, backend_id: &str) -> Result<(), CliError> {
        self.post_empty(&format!("/ctrl/b/{backend_id}/soft-terminate"))
            .await
    }

    pub async fn hard_terminate(&self, backend_id: &str) -> Result<(), CliError> {
        self.post_empty(&format!("/ctrl/b/{backend_id}/hard-terminate"))
            .await
    }

    pub async fn get_snapshot(&self, id: &str) -> Result<serde_json::Value, CliError> {
        self.get(&format!("/ctrl/snapshots/{id}")).await
    }

    pub async fn delete_snapshot(&self, id: &str) -> Result<(), CliError> {
        self.delete_empty(&format!("/ctrl/snapshots/{id}")).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_trailing_slash_stripped() {
        let client = ControllerClient::new("http://localhost:9090/".to_string());
        assert_eq!(client.base_url, "http://localhost:9090");
    }

    #[test]
    fn base_url_no_trailing_slash() {
        let client = ControllerClient::new("http://localhost:9090".to_string());
        assert_eq!(client.base_url, "http://localhost:9090");
    }
}
