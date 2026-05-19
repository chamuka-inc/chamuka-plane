from __future__ import annotations

import asyncio
import threading
from typing import Any

from sandbox_platform.sandbox import AsyncSandbox
from sandbox_platform.types import (
    CreateCredentialBindingRequest,
    CreateSandboxRequest,
    CredentialBinding,
    ListSandboxesResponse,
    NetworkPolicy,
    SandboxData,
    SnapshotRequest,
    SnapshotResponse,
)


def _get_loop() -> asyncio.AbstractEventLoop:
    try:
        asyncio.get_running_loop()
    except RuntimeError:
        pass
    else:
        msg = (
            "Cannot use synchronous Sandbox from within an async context. "
            "Use AsyncSandbox instead."
        )
        raise RuntimeError(msg)

    return asyncio.new_event_loop()


class Sandbox:
    """Synchronous wrapper around AsyncSandbox."""

    def __init__(self, _async: AsyncSandbox) -> None:
        self._async = _async

    @classmethod
    def create(
        cls,
        request: CreateSandboxRequest,
        *,
        control_url: str | None = None,
        token: str | None = None,
    ) -> Sandbox:
        loop = _get_loop()
        try:
            async_sb = loop.run_until_complete(
                AsyncSandbox.create(request, control_url=control_url, token=token)
            )
        finally:
            loop.close()
        return cls(async_sb)

    @classmethod
    def get(
        cls,
        sandbox_id: str,
        *,
        control_url: str | None = None,
        token: str | None = None,
    ) -> Sandbox:
        loop = _get_loop()
        try:
            async_sb = loop.run_until_complete(
                AsyncSandbox.get(sandbox_id, control_url=control_url, token=token)
            )
        finally:
            loop.close()
        return cls(async_sb)

    @classmethod
    def list(
        cls,
        *,
        status: str | None = None,
        limit: int | None = None,
        cursor: str | None = None,
        tag: str | None = None,
        control_url: str | None = None,
        token: str | None = None,
    ) -> ListSandboxesResponse:
        loop = _get_loop()
        try:
            return loop.run_until_complete(
                AsyncSandbox.list(
                    status=status, limit=limit, cursor=cursor, tag=tag,
                    control_url=control_url, token=token,
                )
            )
        finally:
            loop.close()

    @property
    def id(self) -> str:
        return self._async.id

    @property
    def backend_id(self) -> str:
        return self._async.backend_id

    @property
    def status(self) -> str:
        return self._async.status

    @property
    def runtime(self) -> str:
        return self._async.runtime

    def _run(self, coro: Any) -> Any:
        loop = _get_loop()
        try:
            return loop.run_until_complete(coro)
        finally:
            loop.close()

    def run_command(
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
        return self._run(
            self._async.run_command(
                cmd, args=args, env=env, cwd=cwd,
                stdin=stdin, timeout_ms=timeout_ms, user=user,
            )
        )

    def run_command_detached(
        self,
        cmd: str,
        *,
        args: list[str] | None = None,
        env: dict[str, str] | None = None,
        cwd: str | None = None,
        pty: bool = False,
        pty_size: dict | None = None,
    ) -> dict:
        return self._run(
            self._async.run_command_detached(
                cmd, args=args, env=env, cwd=cwd,
                pty=pty, pty_size=pty_size,
            )
        )

    def read_file(self, path: str) -> bytes:
        return self._run(self._async.read_file(path))

    def read_text_file(self, path: str) -> str:
        return self._run(self._async.read_text_file(path))

    def write_file(self, path: str, content: bytes | str) -> None:
        self._run(self._async.write_file(path, content))

    def mkdir(self, path: str, *, recursive: bool = False) -> None:
        self._run(self._async.mkdir(path, recursive=recursive))

    def rm(self, path: str, *, recursive: bool = False) -> None:
        self._run(self._async.rm(path, recursive=recursive))

    def list_dir(self, path: str) -> list[dict]:
        return self._run(self._async.list_dir(path))

    def list_ports(self) -> list[dict]:
        return self._run(self._async.list_ports())

    def snapshot(self, request: SnapshotRequest | None = None) -> SnapshotResponse:
        return self._run(self._async.snapshot(request))

    def set_network_policy(self, policy: NetworkPolicy) -> NetworkPolicy:
        return self._run(self._async.set_network_policy(policy))

    def create_credential(self, binding: CreateCredentialBindingRequest) -> CredentialBinding:
        return self._run(self._async.create_credential(binding))

    def delete_credential(self, credential_id: str) -> None:
        self._run(self._async.delete_credential(credential_id))

    def extend(self, additional_ms: int) -> SandboxData:
        return self._run(self._async.extend(additional_ms))

    def stop(self, *, force: bool = False) -> None:
        self._run(self._async.stop(force=force))

    def close(self) -> None:
        self._run(self._async.close())
