#![allow(clippy::expect_used, clippy::print_stderr)]

use schemars::{schema_for, JsonSchema};
use std::fs;
use std::path::Path;

fn write_schema<T: JsonSchema>(output_dir: &Path) {
    let schema = schema_for!(T);
    let name = T::schema_name();
    let filename = to_filename(&name);
    let json = serde_json::to_string_pretty(&schema).unwrap_or_else(|e| {
        panic!("Failed to serialize schema for {name}: {e}");
    });
    let path = output_dir.join(format!("{filename}.schema.json"));
    fs::write(&path, format!("{json}\n")).unwrap_or_else(|e| {
        panic!("Failed to write {}: {e}", path.display());
    });
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
    use common::ext::network;
    use common::ext::sandbox;
    use common::ext::snapshot;
    use common::ext::usage;

    write_schema::<sandbox::Sandbox>(&output_dir);
    write_schema::<sandbox::TrustLevel>(&output_dir);
    write_schema::<sandbox::Arch>(&output_dir);
    write_schema::<sandbox::SandboxResources>(&output_dir);

    write_schema::<network::NetworkPolicy>(&output_dir);
    write_schema::<network::NetworkRule>(&output_dir);

    write_schema::<snapshot::Snapshot>(&output_dir);

    write_schema::<usage::UsageRecord>(&output_dir);

    write_schema::<api::CreateSandboxRequest>(&output_dir);
    write_schema::<api::CreateSandboxResponse>(&output_dir);
    write_schema::<api::ListSandboxesResponse>(&output_dir);
    write_schema::<api::SnapshotRequest>(&output_dir);
    write_schema::<api::SnapshotResponse>(&output_dir);
    write_schema::<api::ExtendTimeoutRequest>(&output_dir);
    write_schema::<api::CreateSecretRequest>(&output_dir);
    write_schema::<api::CreateSecretResponse>(&output_dir);
    write_schema::<api::CreateCredentialBindingRequest>(&output_dir);
    write_schema::<api::CredentialBinding>(&output_dir);
    write_schema::<api::SandboxConnection>(&output_dir);

    write_schema::<agentd::ExecRequest>(&output_dir);
    write_schema::<agentd::ExecSyncResponse>(&output_dir);
    write_schema::<agentd::ExecDetachedResponse>(&output_dir);
    write_schema::<agentd::LogFrame>(&output_dir);
    write_schema::<agentd::SignalRequest>(&output_dir);
    write_schema::<agentd::MkdirRequest>(&output_dir);
    write_schema::<agentd::ListResponse>(&output_dir);
    write_schema::<agentd::FsEvent>(&output_dir);
    write_schema::<agentd::PortsResponse>(&output_dir);
    write_schema::<agentd::ComputerActionRequest>(&output_dir);
    write_schema::<agentd::ComputerActionResponse>(&output_dir);

    let count = fs::read_dir(&output_dir)
        .expect("read output dir")
        .filter(|e| {
            e.as_ref()
                .map(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                .unwrap_or(false)
        })
        .count();

    eprintln!("Generated {count} schemas in {}", output_dir.display());
}
