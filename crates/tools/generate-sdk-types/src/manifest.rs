/// Each entry maps a schema filename (without `.schema.json`) to the type name
/// it should emit in SDK output. The order here does not affect output order —
/// the registry performs topological sorting.
pub const MANIFEST: &[(&str, &str)] = &[
    // Control-plane types
    ("create-sandbox-request", "CreateSandboxRequest"),
    ("create-sandbox-response", "CreateSandboxResponse"),
    ("get-sandbox-response", "GetSandboxResponse"),
    ("list-sandboxes-response", "ListSandboxesResponse"),
    ("sandbox-connection", "SandboxConnection"),
    ("snapshot-request", "SnapshotRequest"),
    ("snapshot-response", "SnapshotResponse"),
    ("extend-timeout-request", "ExtendTimeoutRequest"),
    ("create-secret-request", "CreateSecretRequest"),
    ("create-secret-response", "CreateSecretResponse"),
    (
        "create-credential-binding-request",
        "CreateCredentialBindingRequest",
    ),
    ("credential-binding", "CredentialBinding"),
    ("sandbox", "Sandbox"),
    ("sandbox-resources", "SandboxResources"),
    ("trust-level", "TrustLevel"),
    ("arch", "Arch"),
    ("snapshot", "Snapshot"),
    ("network-policy", "NetworkPolicy"),
    ("network-rule", "NetworkRule"),
    // Data-plane types
    ("exec-request", "ExecRequest"),
    ("exec-sync-response", "ExecSyncResponse"),
    ("exec-detached-response", "ExecDetachedResponse"),
    ("log-frame", "LogFrame"),
    ("signal-request", "SignalRequest"),
    ("mkdir-request", "MkdirRequest"),
    ("list-response", "ListResponse"),
    ("fs-event", "FsEvent"),
    ("ports-response", "PortsResponse"),
    ("computer-action-request", "ComputerActionRequest"),
    ("computer-action-response", "ComputerActionResponse"),
];
