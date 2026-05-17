from __future__ import annotations

from typing import Literal

from pydantic import BaseModel


BackendStatus = Literal[
    "scheduled",
    "loading",
    "starting",
    "waiting",
    "ready",
    "terminating",
    "hard-terminating",
    "terminated",
]

TrustLevel = Literal["trusted", "untrusted", "paranoid"]

Arch = Literal["amd64", "arm64"]

NetworkPolicyMode = Literal["allow-all", "deny-all", "rules"]

RuleAction = Literal["allow", "deny"]


class SandboxResources(BaseModel):
    vcpus: int
    memory_mb: int | None = None


class NetworkRule(BaseModel):
    domains: list[str] | None = None
    cidrs: list[str] | None = None
    ports: list[int] | None = None
    action: RuleAction | None = None
    inject: str | None = None


class NetworkPolicy(BaseModel):
    mode: NetworkPolicyMode
    rules: list[NetworkRule] | None = None


class SandboxConnection(BaseModel):
    token: str
    url: str
    secret_token: str | None = None
    status_url: str


class SandboxData(BaseModel):
    id: str
    backend_id: str
    tenant: str
    status: BackendStatus
    runtime: str
    trust: TrustLevel
    arch: Arch
    image: str
    resources: SandboxResources
    ports: list[int]
    tags: dict[str, str]
    persistent: bool
    snapshot_source: str | None = None
    created_at: str
    updated_at: str
    expires_at: str
    ready_at: str | None = None
    terminated_at: str | None = None
    drone_name: str | None = None
    cluster: str


class CreateSandboxRequest(BaseModel):
    runtime: str
    image: str | None = None
    trust: TrustLevel | None = None
    arch: Arch | None = None
    resources: SandboxResources
    timeout_ms: int | None = None
    source: dict | None = None
    ports: list[int] | None = None
    env: dict[str, str] | None = None
    tags: dict[str, str] | None = None
    persistent: bool | None = None
    initial_network_policy: NetworkPolicy | None = None
    use_static_token: bool | None = None
    key: dict | None = None


class CreateSandboxResponse(BaseModel):
    sandbox: SandboxData
    connection: SandboxConnection


class ListSandboxesResponse(BaseModel):
    items: list[SandboxData]
    next_cursor: str | None = None


class Snapshot(BaseModel):
    id: str
    sandbox_id: str
    tenant: str | None = None
    image_ref: str | None = None
    image: str | None = None
    digest: str | None = None
    base_image: str | None = None
    size_bytes: int | None = None
    arch: Arch | None = None
    runtime: str
    name: str | None = None
    tag: str | None = None
    tags: dict[str, str] | None = None
    created_at: str
    expires_at: str | None = None


class SnapshotRequest(BaseModel):
    tag: str | None = None
    ttl_days: int | None = None
    tags: dict[str, str] | None = None


class SnapshotResponse(BaseModel):
    snapshot: Snapshot


class ExtendTimeoutRequest(BaseModel):
    additional_ms: int


class CreateCredentialBindingRequest(BaseModel):
    secret_id: str
    env_var: str


class CredentialBinding(BaseModel):
    id: str
    secret_id: str
    env_var: str
    created_at: str


class CreateSecretRequest(BaseModel):
    name: str
    value: str
    description: str | None = None


class CreateSecretResponse(BaseModel):
    id: str
    name: str
    created_at: str
