from __future__ import annotations

from typing import TYPE_CHECKING

from sandbox_platform.http import HttpClient
from sandbox_platform.types import (
    CreateCredentialBindingRequest,
    CreateSandboxResponse,
    CredentialBinding,
    NetworkPolicy,
    SandboxData,
    SnapshotRequest,
    SnapshotResponse,
)

if TYPE_CHECKING:
    from sandbox_platform.client import SandboxClient


class Sandbox:
    def __init__(
        self,
        sandbox_data: SandboxData,
        data_plane_url: str,
        client: SandboxClient | None = None,
    ) -> None:
        self._data = sandbox_data
        self._data_plane = HttpClient(data_plane_url)
        self._client = client

    @classmethod
    def _from_create_response(
        cls,
        response: CreateSandboxResponse,
        client: SandboxClient,
    ) -> "Sandbox":
        return cls(response.sandbox, response.connection.url, client)

    @property
    def id(self) -> str:
        return self._data.id

    @property
    def backend_id(self) -> str:
        return self._data.backend_id

    @property
    def status(self) -> str:
        return self._data.status

    @property
    def runtime(self) -> str:
        return self._data.runtime

    async def run_command(
        self,
        cmd: str,
        *,
        args: list[str] | None = None,
        env: dict[str, str] | None = None,
        cwd: str | None = None,
        stdin: str | None = None,
        timeout_ms: int | None = None,
        user: str | None = None,
    ) -> dict:
        body: dict = {"cmd": cmd, "detached": False}
        if args:
            body["args"] = args
        if env:
            body["env"] = env
        if cwd:
            body["cwd"] = cwd
        if stdin:
            body["stdin"] = stdin
        if timeout_ms:
            body["timeout_ms"] = timeout_ms
        if user:
            body["user"] = user

        return await self._data_plane.request("POST", "/v1/exec", body=body)

    async def spawn(
        self,
        cmd: str,
        *,
        args: list[str] | None = None,
        env: dict[str, str] | None = None,
        cwd: str | None = None,
        pty: bool = False,
        pty_size: dict | None = None,
    ) -> dict:
        body: dict = {"cmd": cmd, "detached": True}
        if args:
            body["args"] = args
        if env:
            body["env"] = env
        if cwd:
            body["cwd"] = cwd
        if pty:
            body["pty"] = pty
        if pty_size:
            body["pty_size"] = pty_size

        return await self._data_plane.request("POST", "/v1/exec", body=body)

    async def get_process_status(self, cmd_id: str) -> dict:
        return await self._data_plane.request("GET", f"/v1/exec/{cmd_id}")

    async def read_file(self, path: str) -> bytes:
        response = await self._data_plane.request_raw("GET", "/v1/fs/read", params={"path": path})
        return response.content

    async def read_text_file(self, path: str) -> str:
        response = await self._data_plane.request_raw("GET", "/v1/fs/read", params={"path": path})
        return response.text

    async def write_file(self, path: str, content: bytes | str) -> None:
        if isinstance(content, str):
            content = content.encode()
        await self._data_plane.request_raw(
            "PUT",
            "/v1/fs/write",
            body={"path": path, "content_base64": __import__("base64").b64encode(content).decode()},
        )

    async def mkdir(self, path: str, *, recursive: bool = False) -> None:
        body: dict = {"path": path}
        if recursive:
            body["recursive"] = True
        await self._data_plane.request("POST", "/v1/fs/mkdir", body=body)

    async def rm(self, path: str, *, recursive: bool = False) -> None:
        params: dict[str, str] = {"path": path}
        if recursive:
            params["recursive"] = "true"
        await self._data_plane.request_raw("DELETE", "/v1/fs/rm", params=params)

    async def list_dir(self, path: str) -> list[dict]:
        data = await self._data_plane.request("GET", "/v1/fs/list", params={"path": path})
        return data["entries"]

    async def list_ports(self) -> list[dict]:
        data = await self._data_plane.request("GET", "/v1/ports")
        return data["listening"]

    async def snapshot(self, request: SnapshotRequest | None = None) -> SnapshotResponse:
        if self._client is None:
            msg = "snapshot() requires a client (create via SandboxClient.create_sandbox)"
            raise RuntimeError(msg)
        return await self._client.snapshot(self.id, request)

    async def set_network_policy(self, policy: NetworkPolicy) -> NetworkPolicy:
        if self._client is None:
            msg = "set_network_policy() requires a client (create via SandboxClient.create_sandbox)"
            raise RuntimeError(msg)
        return await self._client.set_network_policy(self.id, policy)

    async def create_credential(
        self,
        binding: CreateCredentialBindingRequest,
    ) -> CredentialBinding:
        if self._client is None:
            msg = "create_credential() requires a client (create via SandboxClient.create_sandbox)"
            raise RuntimeError(msg)
        return await self._client.create_credential(self.id, binding)

    async def delete_credential(self, credential_id: str) -> None:
        if self._client is None:
            msg = "delete_credential() requires a client (create via SandboxClient.create_sandbox)"
            raise RuntimeError(msg)
        await self._client.delete_credential(self.id, credential_id)

    async def extend(self, additional_ms: int) -> SandboxData:
        if self._client is None:
            msg = "extend() requires a client (create via SandboxClient.create_sandbox)"
            raise RuntimeError(msg)
        return await self._client.extend(self.id, additional_ms)

    async def stop(self, *, force: bool = False) -> None:
        if self._client is None:
            msg = "stop() requires a client (create via SandboxClient.create_sandbox)"
            raise RuntimeError(msg)
        await self._client.stop(self.id, force=force)

    async def close(self) -> None:
        await self._data_plane.close()
