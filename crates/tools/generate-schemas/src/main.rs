#![allow(clippy::expect_used, clippy::print_stderr)]

use schemars::{schema_for, JsonSchema};
use std::fs;
use std::path::Path;

fn write_schema<T: JsonSchema>(output_dir: &Path) -> bool {
    let schema = schema_for!(T);
    let name = T::schema_name();
    let filename = to_filename(&name);
    let json = serde_json::to_string_pretty(&schema).unwrap_or_else(|e| {
        panic!("Failed to serialize schema for {name}: {e}");
    });
    let path = output_dir.join(format!("{filename}.schema.json"));
    let content = format!("{json}\n");
    if fs::read_to_string(&path).ok().as_deref() == Some(&content) {
        return false;
    }
    fs::write(&path, content).unwrap_or_else(|e| {
        panic!("Failed to write {}: {e}", path.display());
    });
    true
}

fn to_filename(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('-');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

fn main() {
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("could not find repo root")
        .join("contracts")
        .join("schemas");

    fs::create_dir_all(&output_dir).unwrap_or_else(|e| {
        panic!("Failed to create {}: {e}", output_dir.display());
    });

    use common::ext::agentd;
    use common::ext::api;
    use common::ext::extension_action;
    use common::ext::network;
    use common::ext::sandbox;
    use common::ext::snapshot;
    use common::ext::usage;

    let mut written = 0u32;
    let mut total = 0u32;

    macro_rules! gen {
        ($ty:ty) => {{
            total += 1;
            if write_schema::<$ty>(&output_dir) {
                written += 1;
            }
        }};
    }

    gen!(sandbox::Sandbox);
    gen!(sandbox::TrustLevel);
    gen!(sandbox::Arch);
    gen!(sandbox::SandboxResources);

    gen!(network::NetworkPolicy);
    gen!(network::NetworkRule);

    gen!(snapshot::Snapshot);

    gen!(usage::UsageRecord);

    gen!(api::CreateSandboxRequest);
    gen!(api::CreateSandboxResponse);
    gen!(api::ListSandboxesResponse);
    gen!(api::SnapshotRequest);
    gen!(api::SnapshotResponse);
    gen!(api::ExtendTimeoutRequest);
    gen!(api::CreateSecretRequest);
    gen!(api::CreateSecretResponse);
    gen!(api::CreateCredentialBindingRequest);
    gen!(api::CredentialBinding);
    gen!(api::SandboxConnection);

    gen!(extension_action::PushNetworkPolicyMessage);
    gen!(extension_action::PushCredentialMessage);
    gen!(extension_action::SnapshotCommand);
    gen!(extension_action::AckExtensionAction);
    gen!(extension_action::ExtensionActionKind);

    gen!(agentd::ExecRequest);
    gen!(agentd::ExecSyncResponse);
    gen!(agentd::ExecDetachedResponse);
    gen!(agentd::LogFrame);
    gen!(agentd::SignalRequest);
    gen!(agentd::MkdirRequest);
    gen!(agentd::ListResponse);
    gen!(agentd::FsEvent);
    gen!(agentd::PortsResponse);
    gen!(agentd::ComputerActionRequest);
    gen!(agentd::ComputerActionResponse);

    eprintln!(
        "Schemas: {written} written, {} unchanged ({total} total)",
        total - written
    );
}
