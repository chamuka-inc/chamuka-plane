from __future__ import annotations

from urllib.parse import quote

from sandbox_platform.http import HttpClient
from sandbox_platform.types import (
    CreateCredentialBindingRequest,
    CreateSandboxRequest,
    CreateSandboxResponse,
    CreateSecretRequest,
    CreateSecretResponse,
    CredentialBinding,
    ExtendTimeoutRequest,
    ListSandboxesResponse,
    NetworkPolicy,
    SandboxData,
    Snapshot,
    SnapshotRequest,
    SnapshotResponse,
)


class SandboxClient:
    def __init__(self, control_url: str, token: str | None = None) -> None:
        self._http = HttpClient(control_url, token)

    async def create(self, request: CreateSandboxRequest) -> CreateSandboxResponse:
        data = await self._http.request(
            "POST",
            "/pub/v1/sandboxes",
            body=request.model_dump(exclude_none=True),
        )
        return CreateSandboxResponse.model_validate(data)

    async def create_sandbox(self, request: CreateSandboxRequest) -> "Sandbox":
        from sandbox_platform.sandbox import Sandbox

        response = await self.create(request)
        return Sandbox._from_create_response(response, self)

    async def list(
        self,
        *,
        status: str | None = None,
        limit: int | None = None,
        cursor: str | None = None,
        tag: str | None = None,
    ) -> ListSandboxesResponse:
        params: dict[str, str] = {}
        if status:
            params["status"] = status
        if limit is not None:
            params["limit"] = str(limit)
        if cursor:
            params["cursor"] = cursor
        if tag:
            params["tag"] = tag

        data = await self._http.request("GET", "/pub/v1/sandboxes", params=params)
        return ListSandboxesResponse.model_validate(data)

    async def get(self, sandbox_id: str) -> SandboxData:
        data = await self._http.request("GET", f"/pub/v1/sandboxes/{quote(sandbox_id)}")
        return SandboxData.model_validate(data)

    async def snapshot(
        self,
        sandbox_id: str,
        request: SnapshotRequest | None = None,
    ) -> SnapshotResponse:
        body = request.model_dump(exclude_none=True) if request else {}
        data = await self._http.request(
            "POST",
            f"/pub/v1/sandboxes/{quote(sandbox_id)}/snapshot",
            body=body,
        )
        return SnapshotResponse.model_validate(data)

    async def set_network_policy(
        self,
        sandbox_id: str,
        policy: NetworkPolicy,
    ) -> NetworkPolicy:
        data = await self._http.request(
            "POST",
            f"/pub/v1/sandboxes/{quote(sandbox_id)}/network-policy",
            body=policy.model_dump(exclude_none=True),
        )
        return NetworkPolicy.model_validate(data)

    async def create_credential(
        self,
        sandbox_id: str,
        binding: CreateCredentialBindingRequest,
    ) -> CredentialBinding:
        data = await self._http.request(
            "POST",
            f"/pub/v1/sandboxes/{quote(sandbox_id)}/credentials",
            body=binding.model_dump(exclude_none=True),
        )
        return CredentialBinding.model_validate(data)

    async def delete_credential(self, sandbox_id: str, credential_id: str) -> None:
        await self._http.request(
            "DELETE",
            f"/pub/v1/sandboxes/{quote(sandbox_id)}/credentials/{quote(credential_id)}",
        )

    async def extend(self, sandbox_id: str, additional_ms: int) -> SandboxData:
        body = ExtendTimeoutRequest(additional_ms=additional_ms)
        data = await self._http.request(
            "POST",
            f"/pub/v1/sandboxes/{quote(sandbox_id)}/extend",
            body=body.model_dump(),
        )
        return SandboxData.model_validate(data)

    async def stop(self, sandbox_id: str, *, force: bool = False) -> None:
        params: dict[str, str] = {}
        if force:
            params["force"] = "true"
        await self._http.request(
            "DELETE",
            f"/pub/v1/sandboxes/{quote(sandbox_id)}",
            params=params or None,
        )

    async def get_snapshot(self, snapshot_id: str) -> Snapshot:
        data = await self._http.request("GET", f"/pub/v1/snapshots/{quote(snapshot_id)}")
        return Snapshot.model_validate(data)

    async def delete_snapshot(self, snapshot_id: str) -> None:
        await self._http.request("DELETE", f"/pub/v1/snapshots/{quote(snapshot_id)}")

    async def create_secret(self, request: CreateSecretRequest) -> CreateSecretResponse:
        data = await self._http.request(
            "POST",
            "/pub/v1/secrets",
            body=request.model_dump(exclude_none=True),
        )
        return CreateSecretResponse.model_validate(data)

    async def delete_secret(self, secret_id: str) -> None:
        await self._http.request("DELETE", f"/pub/v1/secrets/{quote(secret_id)}")

    async def close(self) -> None:
        await self._http.close()

    async def __aenter__(self) -> "SandboxClient":
        return self

    async def __aexit__(self, *_: object) -> None:
        await self.close()
