import httpx
import pytest
import respx

from sandbox_platform.client import SandboxClient
from sandbox_platform.types import (
    CreateCredentialBindingRequest,
    CreateSandboxRequest,
    NetworkPolicy,
    SandboxResources,
    SnapshotRequest,
)

CONTROL_URL = "http://localhost:9900"
DATA_PLANE_URL = "http://localhost:9901/tk-abc123"

MOCK_SANDBOX = {
    "id": "sbx-test001",
    "backend_id": "bk-test001",
    "tenant": "tenant-abc",
    "status": "ready",
    "runtime": "node24",
    "trust": "untrusted",
    "arch": "amd64",
    "image": "node:24-slim",
    "resources": {"vcpus": 2, "memory_mb": 4096},
    "ports": [3000],
    "tags": {},
    "persistent": False,
    "created_at": "2026-05-17T10:00:00Z",
    "updated_at": "2026-05-17T10:00:05Z",
    "expires_at": "2026-05-17T10:05:00Z",
    "ready_at": "2026-05-17T10:00:05Z",
    "cluster": "default",
}

MOCK_CREATE_RESPONSE = {
    "sandbox": MOCK_SANDBOX,
    "connection": {
        "token": "tk-abc123",
        "url": f"{DATA_PLANE_URL}/",
        "status_url": f"{CONTROL_URL}/pub/v1/sandboxes/sbx-test001/status",
    },
}

MOCK_SNAPSHOT = {
    "id": "snap-001",
    "sandbox_id": "sbx-test001",
    "image": "registry.internal/snap-001:latest",
    "runtime": "node24",
    "tag": "v1",
    "size_bytes": 2048,
    "created_at": "2026-05-17T10:01:00Z",
}

MOCK_CREDENTIAL = {
    "id": "cred-bind-001",
    "secret_id": "secret-abc",
    "env_var": "API_KEY",
    "created_at": "2026-05-17T10:02:00Z",
}


@pytest.fixture()
def mock_control():
    with respx.mock(base_url=CONTROL_URL, assert_all_called=False) as respx_mock:
        respx_mock.post("/pub/v1/sandboxes").mock(
            return_value=httpx.Response(201, json=MOCK_CREATE_RESPONSE)
        )
        respx_mock.post("/pub/v1/sandboxes/sbx-test001/snapshot").mock(
            return_value=httpx.Response(201, json={"snapshot": MOCK_SNAPSHOT})
        )
        respx_mock.post("/pub/v1/sandboxes/sbx-test001/network-policy").mock(
            return_value=httpx.Response(200, json={"mode": "deny-all"})
        )
        respx_mock.post("/pub/v1/sandboxes/sbx-test001/credentials").mock(
            return_value=httpx.Response(201, json=MOCK_CREDENTIAL)
        )
        respx_mock.delete("/pub/v1/sandboxes/sbx-test001/credentials/cred-bind-001").mock(
            return_value=httpx.Response(204)
        )
        respx_mock.post("/pub/v1/sandboxes/sbx-test001/extend").mock(
            return_value=httpx.Response(200, json=MOCK_SANDBOX)
        )
        respx_mock.delete("/pub/v1/sandboxes/sbx-test001").mock(
            return_value=httpx.Response(204)
        )
        yield respx_mock


async def _create_sandbox(mock_control: respx.MockRouter):
    client = SandboxClient(CONTROL_URL, "test-token")
    return await client.create_sandbox(
        CreateSandboxRequest(runtime="node24", resources=SandboxResources(vcpus=2))
    )


class TestSandboxProperties:
    async def test_exposes_metadata(self, mock_control: respx.MockRouter) -> None:
        sandbox = await _create_sandbox(mock_control)
        assert sandbox.id == "sbx-test001"
        assert sandbox.backend_id == "bk-test001"
        assert sandbox.status == "ready"
        assert sandbox.runtime == "node24"


class TestSandboxControlMethods:
    async def test_snapshot(self, mock_control: respx.MockRouter) -> None:
        sandbox = await _create_sandbox(mock_control)
        result = await sandbox.snapshot(SnapshotRequest(tag="v1"))
        assert result.snapshot.id == "snap-001"

    async def test_set_network_policy(self, mock_control: respx.MockRouter) -> None:
        sandbox = await _create_sandbox(mock_control)
        result = await sandbox.set_network_policy(NetworkPolicy(mode="deny-all"))
        assert result.mode == "deny-all"

    async def test_create_credential(self, mock_control: respx.MockRouter) -> None:
        sandbox = await _create_sandbox(mock_control)
        result = await sandbox.create_credential(
            CreateCredentialBindingRequest(secret_id="secret-abc", env_var="API_KEY")
        )
        assert result.id == "cred-bind-001"

    async def test_delete_credential(self, mock_control: respx.MockRouter) -> None:
        sandbox = await _create_sandbox(mock_control)
        await sandbox.delete_credential("cred-bind-001")

    async def test_extend(self, mock_control: respx.MockRouter) -> None:
        sandbox = await _create_sandbox(mock_control)
        result = await sandbox.extend(60_000)
        assert result.id == "sbx-test001"

    async def test_stop(self, mock_control: respx.MockRouter) -> None:
        sandbox = await _create_sandbox(mock_control)
        await sandbox.stop()

    async def test_stop_force(self, mock_control: respx.MockRouter) -> None:
        sandbox = await _create_sandbox(mock_control)
        await sandbox.stop(force=True)
