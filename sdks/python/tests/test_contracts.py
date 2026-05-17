import json
from pathlib import Path

import httpx
import pytest
import respx

from sandbox_platform.client import SandboxClient
from sandbox_platform.types import (
    CreateSandboxRequest,
    NetworkPolicy,
)

FIXTURES_DIR = Path(__file__).parent.parent.parent.parent / "contracts" / "fixtures"
CONTROL_URL = "http://localhost:9900"


def load_fixture(path: str) -> dict:
    return json.loads((FIXTURES_DIR / path).read_text())


@pytest.fixture()
def mock_api():
    with respx.mock(base_url=CONTROL_URL, assert_all_called=False) as respx_mock:
        yield respx_mock


class TestCreateSandboxFixtures:
    async def test_sends_full_request_fixture(self, mock_api: respx.MockRouter) -> None:
        request_fixture = load_fixture("api/create-sandbox-request.full.json")
        response_fixture = load_fixture("api/create-sandbox-response.json")

        route = mock_api.post("/pub/v1/sandboxes").mock(
            return_value=httpx.Response(201, json=response_fixture)
        )

        async with SandboxClient(CONTROL_URL, "test") as client:
            await client.create(CreateSandboxRequest.model_validate(request_fixture))

        body = json.loads(route.calls.last.request.content)
        assert body["runtime"] == request_fixture["runtime"]
        assert body["image"] == request_fixture["image"]
        assert body["trust"] == request_fixture["trust"]
        assert body["arch"] == request_fixture["arch"]
        assert body["timeout_ms"] == request_fixture["timeout_ms"]
        assert body["persistent"] == request_fixture["persistent"]
        assert body["use_static_token"] == request_fixture["use_static_token"]

    async def test_parses_response_fixture(self, mock_api: respx.MockRouter) -> None:
        response_fixture = load_fixture("api/create-sandbox-response.json")

        mock_api.post("/pub/v1/sandboxes").mock(
            return_value=httpx.Response(201, json=response_fixture)
        )

        async with SandboxClient(CONTROL_URL, "test") as client:
            result = await client.create(
                CreateSandboxRequest.model_validate({"runtime": "node24", "resources": {"vcpus": 2}})
            )

        assert result.sandbox.id == response_fixture["sandbox"]["id"]
        assert result.sandbox.backend_id == response_fixture["sandbox"]["backend_id"]
        assert result.connection.token == response_fixture["connection"]["token"]
        assert result.connection.url == response_fixture["connection"]["url"]


class TestSandboxFixtures:
    async def test_list_parses_fixture(self, mock_api: respx.MockRouter) -> None:
        sandbox_fixture = load_fixture("sandbox/full.json")

        mock_api.get("/pub/v1/sandboxes").mock(
            return_value=httpx.Response(200, json={"items": [sandbox_fixture]})
        )

        async with SandboxClient(CONTROL_URL, "test") as client:
            result = await client.list()

        assert len(result.items) == 1
        assert result.items[0].id == sandbox_fixture["id"]
        assert result.items[0].runtime == sandbox_fixture["runtime"]
        assert result.items[0].trust == sandbox_fixture["trust"]
        assert result.items[0].persistent == sandbox_fixture["persistent"]


class TestNetworkPolicyFixtures:
    async def test_sends_rules_fixture(self, mock_api: respx.MockRouter) -> None:
        fixture = load_fixture("network/rules.json")

        route = mock_api.post("/pub/v1/sandboxes/sbx-test/network-policy").mock(
            return_value=httpx.Response(200, json=fixture)
        )

        async with SandboxClient(CONTROL_URL, "test") as client:
            result = await client.set_network_policy(
                "sbx-test", NetworkPolicy.model_validate(fixture)
            )

        body = json.loads(route.calls.last.request.content)
        assert body["mode"] == "rules"
        assert len(body["rules"]) == len(fixture["rules"])
        assert result.mode == "rules"

    async def test_sends_allow_all_fixture(self, mock_api: respx.MockRouter) -> None:
        fixture = load_fixture("network/allow-all.json")

        route = mock_api.post("/pub/v1/sandboxes/sbx-test/network-policy").mock(
            return_value=httpx.Response(200, json=fixture)
        )

        async with SandboxClient(CONTROL_URL, "test") as client:
            await client.set_network_policy("sbx-test", NetworkPolicy.model_validate(fixture))

        body = json.loads(route.calls.last.request.content)
        assert body["mode"] == "allow-all"

    async def test_sends_deny_all_fixture(self, mock_api: respx.MockRouter) -> None:
        fixture = load_fixture("network/deny-all.json")

        route = mock_api.post("/pub/v1/sandboxes/sbx-test/network-policy").mock(
            return_value=httpx.Response(200, json=fixture)
        )

        async with SandboxClient(CONTROL_URL, "test") as client:
            await client.set_network_policy("sbx-test", NetworkPolicy.model_validate(fixture))

        body = json.loads(route.calls.last.request.content)
        assert body["mode"] == "deny-all"


class TestSnapshotFixtures:
    async def test_parses_snapshot_fixture(self, mock_api: respx.MockRouter) -> None:
        fixture = load_fixture("snapshot/full.json")

        mock_api.get("/pub/v1/snapshots/snap-abc123").mock(
            return_value=httpx.Response(200, json=fixture)
        )

        async with SandboxClient(CONTROL_URL, "test") as client:
            result = await client.get_snapshot("snap-abc123")

        assert result.id == fixture["id"]
        assert result.sandbox_id == fixture["sandbox_id"]
        assert result.image_ref == fixture["image_ref"]
        assert result.runtime == fixture["runtime"]
        assert result.size_bytes == fixture["size_bytes"]
        assert result.name == fixture["name"]
