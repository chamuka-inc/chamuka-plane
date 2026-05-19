import httpx
import pytest
import respx

from sandbox_platform.client import SandboxClient
from sandbox_platform.types import (
    CreateCredentialBindingRequest,
    CreateSandboxRequest,
    CreateSecretRequest,
    NetworkPolicy,
    NetworkRule,
    ResourcesSpec,
    SandboxResources,
    SnapshotRequest,
)

CONTROL_URL = "http://localhost:9900"

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
    "tags": {"env": "dev"},
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
        "url": "http://localhost:9901/tk-abc123/",
        "status_url": f"{CONTROL_URL}/pub/v1/sandboxes/sbx-test001/status",
    },
}

MOCK_SNAPSHOT = {
    "id": "snap-001",
    "sandbox_id": "sbx-test001",
    "tenant": "tenant-abc",
    "image_ref": "registry.internal/snap-001:latest",
    "digest": "sha256:abc123",
    "base_image": "node:24-slim",
    "arch": "amd64",
    "runtime": "node24",
    "name": "checkpoint-1",
    "size_bytes": 1048576,
    "created_at": "2026-05-17T10:01:00Z",
    "expires_at": "2026-06-17T10:01:00Z",
}

MOCK_CREDENTIAL = {
    "id": "cred-bind-001",
    "sandbox_id": "sbx-test001",
    "destination": "https://api.example.com",
    "header_name": "Authorization",
    "header_format": "Bearer {value}",
    "secret_id": "secret-abc",
    "created_at": "2026-05-17T10:02:00Z",
}

MOCK_CONNECTION = {
    "token": "tk-abc123",
    "url": "http://localhost:9901/tk-abc123/",
    "status_url": f"{CONTROL_URL}/pub/v1/sandboxes/sbx-test001/status",
}

MOCK_GET_RESPONSE = {
    "sandbox": MOCK_SANDBOX,
    "connection": MOCK_CONNECTION,
}

MOCK_SECRET = {
    "id": "secret-new001",
    "name": "my-api-key",
    "created_at": "2026-05-17T10:03:00Z",
}


@pytest.fixture()
def mock_api():
    with respx.mock(base_url=CONTROL_URL, assert_all_called=False) as respx_mock:
        yield respx_mock


class TestCreate:
    async def test_creates_sandbox(self, mock_api: respx.MockRouter) -> None:
        mock_api.post("/pub/v1/sandboxes").mock(
            return_value=httpx.Response(201, json=MOCK_CREATE_RESPONSE)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            result = await client.create(
                CreateSandboxRequest(runtime="node24", resources=ResourcesSpec(vcpus=2))
            )

        assert result.sandbox.id == "sbx-test001"
        assert result.sandbox.status == "ready"
        assert result.connection.token == "tk-abc123"

    async def test_passes_full_request_body(self, mock_api: respx.MockRouter) -> None:
        route = mock_api.post("/pub/v1/sandboxes").mock(
            return_value=httpx.Response(201, json=MOCK_CREATE_RESPONSE)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            await client.create(
                CreateSandboxRequest(
                    runtime="python313",
                    trust="paranoid",
                    resources=ResourcesSpec(vcpus=4),
                    env={"NODE_ENV": "production"},
                    timeout_ms=600_000,
                    persistent=True,
                    tags={"team": "backend"},
                )
            )

        body = route.calls.last.request.content
        import json

        parsed = json.loads(body)
        assert parsed["runtime"] == "python313"
        assert parsed["trust"] == "paranoid"
        assert parsed["resources"] == {"vcpus": 4}
        assert parsed["persistent"] is True


class TestList:
    async def test_returns_paginated_list(self, mock_api: respx.MockRouter) -> None:
        mock_api.get("/pub/v1/sandboxes").mock(
            return_value=httpx.Response(
                200, json={"items": [MOCK_SANDBOX], "next_cursor": "cursor-page2"}
            )
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            result = await client.list()

        assert len(result.items) == 1
        assert result.items[0].id == "sbx-test001"
        assert result.next_cursor == "cursor-page2"

    async def test_passes_query_params(self, mock_api: respx.MockRouter) -> None:
        route = mock_api.get("/pub/v1/sandboxes").mock(
            return_value=httpx.Response(200, json={"items": []})
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            await client.list(status="ready", limit=50, cursor="abc")

        request = route.calls.last.request
        assert request.url.params["status"] == "ready"
        assert request.url.params["limit"] == "50"
        assert request.url.params["cursor"] == "abc"


class TestGet:
    async def test_returns_sandbox_by_id(self, mock_api: respx.MockRouter) -> None:
        mock_api.get("/pub/v1/sandboxes/sbx-test001").mock(
            return_value=httpx.Response(200, json=MOCK_GET_RESPONSE)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            sandbox = await client.get("sbx-test001")

        assert sandbox.id == "sbx-test001"
        assert sandbox.runtime == "node24"

    async def test_get_with_connection(self, mock_api: respx.MockRouter) -> None:
        mock_api.get("/pub/v1/sandboxes/sbx-test001").mock(
            return_value=httpx.Response(200, json=MOCK_GET_RESPONSE)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            response = await client.get_with_connection("sbx-test001")

        assert response.sandbox.id == "sbx-test001"
        assert response.sandbox.runtime == "node24"
        assert response.connection is not None
        assert response.connection.token == "tk-abc123"
        assert response.connection.url == "http://localhost:9901/tk-abc123/"

    async def test_get_without_connection(self, mock_api: respx.MockRouter) -> None:
        mock_api.get("/pub/v1/sandboxes/sbx-test001").mock(
            return_value=httpx.Response(200, json={"sandbox": MOCK_SANDBOX})
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            response = await client.get_with_connection("sbx-test001")

        assert response.sandbox.id == "sbx-test001"
        assert response.connection is None

    async def test_raises_on_404(self, mock_api: respx.MockRouter) -> None:
        mock_api.get("/pub/v1/sandboxes/sbx-missing").mock(
            return_value=httpx.Response(404, json={"message": "Not found"})
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            with pytest.raises(Exception):
                await client.get("sbx-missing")


class TestSnapshot:
    async def test_creates_snapshot(self, mock_api: respx.MockRouter) -> None:
        mock_api.post("/pub/v1/sandboxes/sbx-test001/snapshot").mock(
            return_value=httpx.Response(201, json={"snapshot": MOCK_SNAPSHOT})
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            result = await client.snapshot(
                "sbx-test001", SnapshotRequest(name="checkpoint-1")
            )

        assert result.snapshot.id == "snap-001"
        assert result.snapshot.sandbox_id == "sbx-test001"


class TestNetworkPolicy:
    async def test_updates_policy(self, mock_api: respx.MockRouter) -> None:
        policy_data = {"mode": "rules", "rules": [{"domains": ["api.example.com"], "action": "allow"}]}
        mock_api.post("/pub/v1/sandboxes/sbx-test001/network-policy").mock(
            return_value=httpx.Response(200, json=policy_data)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            result = await client.set_network_policy(
                "sbx-test001",
                NetworkPolicy(
                    mode="rules",
                    rules=[NetworkRule(domains=["api.example.com"], action="allow")],
                ),
            )

        assert result.mode == "rules"


class TestCredentials:
    async def test_creates_credential(self, mock_api: respx.MockRouter) -> None:
        mock_api.post("/pub/v1/sandboxes/sbx-test001/credentials").mock(
            return_value=httpx.Response(201, json=MOCK_CREDENTIAL)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            result = await client.create_credential(
                "sbx-test001",
                CreateCredentialBindingRequest(
                    secret_id="secret-abc",
                    destination="https://api.example.com",
                    header_name="Authorization",
                ),
            )

        assert result.id == "cred-bind-001"

    async def test_deletes_credential(self, mock_api: respx.MockRouter) -> None:
        mock_api.delete("/pub/v1/sandboxes/sbx-test001/credentials/cred-bind-001").mock(
            return_value=httpx.Response(204)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            await client.delete_credential("sbx-test001", "cred-bind-001")


class TestExtend:
    async def test_extends_lifetime(self, mock_api: respx.MockRouter) -> None:
        route = mock_api.post("/pub/v1/sandboxes/sbx-test001/extend").mock(
            return_value=httpx.Response(200, json=MOCK_SANDBOX)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            result = await client.extend("sbx-test001", 300_000)

        assert result.id == "sbx-test001"
        import json

        body = json.loads(route.calls.last.request.content)
        assert body["additional_ms"] == 300_000


class TestStop:
    async def test_stops_sandbox(self, mock_api: respx.MockRouter) -> None:
        route = mock_api.delete("/pub/v1/sandboxes/sbx-test001").mock(
            return_value=httpx.Response(204)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            await client.stop("sbx-test001")

        assert "force" not in str(route.calls.last.request.url)

    async def test_stops_with_force(self, mock_api: respx.MockRouter) -> None:
        route = mock_api.delete("/pub/v1/sandboxes/sbx-test001").mock(
            return_value=httpx.Response(204)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            await client.stop("sbx-test001", force=True)

        assert "force=true" in str(route.calls.last.request.url)


class TestSnapshots:
    async def test_gets_snapshot(self, mock_api: respx.MockRouter) -> None:
        mock_api.get("/pub/v1/snapshots/snap-001").mock(
            return_value=httpx.Response(200, json=MOCK_SNAPSHOT)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            result = await client.get_snapshot("snap-001")

        assert result.id == "snap-001"
        assert result.sandbox_id == "sbx-test001"

    async def test_deletes_snapshot(self, mock_api: respx.MockRouter) -> None:
        mock_api.delete("/pub/v1/snapshots/snap-001").mock(
            return_value=httpx.Response(204)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            await client.delete_snapshot("snap-001")


class TestSecrets:
    async def test_creates_secret(self, mock_api: respx.MockRouter) -> None:
        route = mock_api.post("/pub/v1/secrets").mock(
            return_value=httpx.Response(201, json=MOCK_SECRET)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            result = await client.create_secret(
                CreateSecretRequest(name="my-api-key", value="sk-secret-value")
            )

        assert result.id == "secret-new001"
        import json

        body = json.loads(route.calls.last.request.content)
        assert body["name"] == "my-api-key"
        assert body["value"] == "sk-secret-value"

    async def test_deletes_secret(self, mock_api: respx.MockRouter) -> None:
        mock_api.delete("/pub/v1/secrets/secret-abc").mock(
            return_value=httpx.Response(204)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            await client.delete_secret("secret-abc")


class TestAuth:
    async def test_sends_bearer_token(self, mock_api: respx.MockRouter) -> None:
        route = mock_api.get("/pub/v1/sandboxes/sbx-test001").mock(
            return_value=httpx.Response(200, json=MOCK_GET_RESPONSE)
        )
        async with SandboxClient(CONTROL_URL, "test-token") as client:
            await client.get("sbx-test001")

        assert route.calls.last.request.headers["Authorization"] == "Bearer test-token"
