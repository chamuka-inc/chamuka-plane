pub use chamuka_common::ext::api::{
    CreateSandboxRequest, CreateSandboxResponse, KeySpec, ResourcesSpec, SandboxConnection,
    SandboxSource,
};
pub use chamuka_common::ext::extension_action::{
    CredentialPayload, PushCredentialMessage, PushNetworkPolicyMessage, SnapshotCommand,
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

pub struct ExtensionActionId(pub String);

impl ExtensionActionId {
    pub fn new() -> Self {
        Self(format!("xak-{}", plane_common::util::random_string()))
    }
}

impl std::fmt::Display for ExtensionActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for ExtensionActionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
