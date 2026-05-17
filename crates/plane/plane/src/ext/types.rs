pub use chamuka_common::ext::api::{
    CreateSandboxRequest, CreateSandboxResponse, KeySpec, ResourcesSpec, SandboxConnection,
    SandboxSource,
};
pub use chamuka_common::ext::network::NetworkPolicy;
pub use chamuka_common::ext::sandbox::{Arch, Sandbox, SandboxResources, TrustLevel};

pub struct SandboxId(pub String);

impl SandboxId {
    pub fn new() -> Self {
        Self(format!("sb-{}", plane_common::util::random_string()))
    }
}

impl std::fmt::Display for SandboxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SandboxId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
