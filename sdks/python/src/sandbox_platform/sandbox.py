from __future__ import annotations

import base64
from typing import TYPE_CHECKING, Any

from sandbox_platform.auth import resolve_auth
from sandbox_platform.http import HttpClient
from sandbox_platform.types import (
    CreateCredentialBindingRequest,
    CreateSandboxRequest,
    CreateSandboxResponse,
    CredentialBinding,
    GetSandboxResponse,
    ListSandboxesResponse,
    NetworkPolicy,
    SandboxData,
    SnapshotRequest,
    SnapshotResponse,
)

if TYPE_CHECKING:
    from sandbox_platform.client import SandboxClient


class AsyncSandbox:
    def __init__(
        self,
        sandbox_data: SandboxData,
        data_plane_url: str,
        client: SandboxClient | None = None,
    ) -> None:
        self._data = sandbox_data
        self._data_plane = HttpClient(data_plane_url) if data_plane_url else None
        self._client = client

    @classmethod
    def _from_create_response(
        cls,
        response: CreateSandboxResponse,
        client: SandboxClient,
    ) -> AsyncSandbox:
        return cls(response.sandbox, response.connection.url, client)

    # --- Static factory methods ---

    @classmethod
    async def create(
        cls,
        request: CreateSandboxRequest,
        *,
        control_url: str | None = None,
        token: str | None = None,
    ) -> AsyncSandbox:
        from sandbox_platform.client import SandboxClient

        url, resolved_token = resolve_auth(control_url=control_url, token=token)
        client = SandboxClient(url, resolved_token)
        response = await client.create(request)
        return cls._from_create_response(response, client)

    @classmethod
    async def get(
        cls,
        sandbox_id: str,
        *,
        control_url: str | None = None,
        token: str | None = None,
    ) -> AsyncSandbox:
        from sandbox_platform.client import SandboxClient

        url, resolved_token = resolve_auth(control_url=control_url, token=token)
        client = SandboxClient(url, resolved_token)
        response = await client.get_with_connection(sandbox_id)
        connection = response.connection
        data_plane_url = connection.url if connection else ""
        instance = cls(response.sandbox, data_plane_url, client)
        return instance

    @classmethod
    async def list(
        cls,
        *,
        status: str | None = None,
        limit: int | None = None,
        cursor: str | None = None,
        tag: str | None = None,
        control_url: str | None = None,
        token: str | None = None,
    ) -> ListSandboxesResponse:
        from sandbox_platform.client import SandboxClient

        url, resolved_token = resolve_auth(control_url=control_url, token=token)
        client = SandboxClient(url, resolved_token)
        return await client.list(status=status, limit=limit, cursor=cursor, tag=tag)

    # --- Properties ---

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

    # --- Internals ---

    def _require_client(self, method: str) -> SandboxClient:
        if self._client is None:
            msg = f"{method}() requires a managed sandbox (create via AsyncSandbox.create())"
            raise RuntimeError(msg)
        return self._client

    def _require_data_plane(self, method: str) -> HttpClient:
        if self._data_plane is None:
            msg = (
                f"{method}() requires an active data-plane connection. "
                "The sandbox may be terminated or was fetched without a connection."
            )
            raise RuntimeError(msg)
        return self._data_plane

    # --- Command execution ---

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
        dp = self._require_data_plane("run_command")
        body = _build_exec_body(
            cmd, detached=False, args=args, env=env, cwd=cwd,
            stdin=stdin, timeout_ms=timeout_ms, user=user,
        )
        return await dp.request("POST", "/v1/exec", body=body)

    async def run_command_detached(
        self,
        cmd: str,
        *,
        args: list[str] | None = None,
        env: dict[str, str] | None = None,
        cwd: str | None = None,
        pty: bool = False,
        pty_size: dict | None = None,
    ) -> dict:
        dp = self._require_data_plane("run_command_detached")
        body = _build_exec_body(cmd, detached=True, args=args, env=env, cwd=cwd)
        if pty:
            body["pty"] = pty
        if pty_size:
            body["pty_size"] = pty_size
        return await dp.request("POST", "/v1/exec", body=body)

    async def get_process_status(self, cmd_id: str) -> dict:
        dp = self._require_data_plane("get_process_status")
        return await dp.request("GET", f"/v1/exec/{cmd_id}")

    # --- Filesystem ---

    async def read_file(self, path: str) -> bytes:
        dp = self._require_data_plane("read_file")
        response = await dp.request_raw("GET", "/v1/fs/read", params={"path": path})
        return response.content

    async def read_text_file(self, path: str) -> str:
        dp = self._require_data_plane("read_text_file")
        response = await dp.request_raw("GET", "/v1/fs/read", params={"path": path})
        return response.text

    async def write_file(self, path: str, content: bytes | str) -> None:
        dp = self._require_data_plane("write_file")
        if isinstance(content, str):
            content = content.encode()
        await dp.request_raw(
            "PUT",
            "/v1/fs/write",
            body={"path": path, "content_base64": base64.b64encode(content).decode()},
        )

    async def mkdir(self, path: str, *, recursive: bool = False) -> None:
        dp = self._require_data_plane("mkdir")
        body: dict = {"path": path}
        if recursive:
            body["recursive"] = True
        await dp.request("POST", "/v1/fs/mkdir", body=body)

    async def rm(self, path: str, *, recursive: bool = False) -> None:
        dp = self._require_data_plane("rm")
        params: dict[str, str] = {"path": path}
        if recursive:
            params["recursive"] = "true"
        await dp.request_raw("DELETE", "/v1/fs/rm", params=params)

    async def list_dir(self, path: str) -> list[dict]:
        dp = self._require_data_plane("list_dir")
        data = await dp.request("GET", "/v1/fs/list", params={"path": path})
        return data["entries"]

    async def list_ports(self) -> list[dict]:
        dp = self._require_data_plane("list_ports")
        data = await dp.request("GET", "/v1/ports")
        return data["listening"]

    # --- Control-plane delegation ---

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
        if self._data_plane:
            await self._data_plane.close()

    async def __aenter__(self) -> AsyncSandbox:
        return self

    async def __aexit__(self, *_: object) -> None:
        await self.close()


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
