pub use chamuka_common::ext::api::{
    CreateSandboxRequest, CreateSandboxResponse, KeySpec, ResourcesSpec, SandboxConnection,
    SandboxSource,
};
pub use chamuka_common::ext::extension_action::{
    CredentialPayload, ExtensionActionKind, PushCredentialMessage, PushNetworkPolicyMessage,
    SnapshotCommand,
};
pub use chamuka_common::ext::network::NetworkPolicy;
pub use chamuka_common::ext::sandbox::{Arch, Sandbox, SandboxResources, TrustLevel};

macro_rules! prefixed_id {
    ($name:ident, $prefix:literal) => {
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self {
                Self(format!(
                    concat!($prefix, "{}"),
                    plane_common::util::random_string()
                ))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

prefixed_id!(SandboxId, "sb-");
prefixed_id!(ExtensionActionId, "xak-");
