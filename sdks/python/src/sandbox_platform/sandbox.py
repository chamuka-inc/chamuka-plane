from __future__ import annotations

import base64
from typing import TYPE_CHECKING, Any

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


def _build_exec_body(
    cmd: str,
    *,
    detached: bool,
    args: list[str] | None = None,
    env: dict[str, str] | None = None,
    cwd: str | None = None,
    stdin: str | None = None,
    timeout_ms: int | None = None,
    user: str | None = None,
) -> dict[str, Any]:
    body: dict[str, Any] = {"cmd": cmd, "detached": detached}
    if args is not None:
        body["args"] = args
    if env is not None:
        body["env"] = env
    if cwd is not None:
        body["cwd"] = cwd
    if stdin is not None:
        body["stdin"] = stdin
    if timeout_ms is not None:
        body["timeout_ms"] = timeout_ms
    if user is not None:
        body["user"] = user
    return body


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
    ) -> Sandbox:
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

    def _require_client(self, method: str) -> SandboxClient:
        if self._client is None:
            msg = f"{method}() requires a client (create via SandboxClient.create_sandbox)"
            raise RuntimeError(msg)
        return self._client

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
        body = _build_exec_body(
            cmd, detached=False, args=args, env=env, cwd=cwd,
            stdin=stdin, timeout_ms=timeout_ms, user=user,
        )
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
        body = _build_exec_body(cmd, detached=True, args=args, env=env, cwd=cwd)
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
            body={"path": path, "content_base64": base64.b64encode(content).decode()},
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
        client = self._require_client("snapshot")
        return await client.snapshot(self.id, request)

    async def set_network_policy(self, policy: NetworkPolicy) -> NetworkPolicy:
        client = self._require_client("set_network_policy")
        return await client.set_network_policy(self.id, policy)

    async def create_credential(
        self,
        binding: CreateCredentialBindingRequest,
    ) -> CredentialBinding:
        client = self._require_client("create_credential")
        return await client.create_credential(self.id, binding)

    async def delete_credential(self, credential_id: str) -> None:
        client = self._require_client("delete_credential")
        await client.delete_credential(self.id, credential_id)

    async def extend(self, additional_ms: int) -> SandboxData:
        client = self._require_client("extend")
        return await client.extend(self.id, additional_ms)

    async def stop(self, *, force: bool = False) -> None:
        client = self._require_client("stop")
        await client.stop(self.id, force=force)

    async def close(self) -> None:
        await self._data_plane.close()

    async def __aenter__(self) -> Sandbox:
        return self

    async def __aexit__(self, *_: object) -> None:
        await self.close()
