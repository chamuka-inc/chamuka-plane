#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
