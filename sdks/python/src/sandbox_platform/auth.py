from __future__ import annotations

import os
from dataclasses import dataclass

from sandbox_platform.errors import SandboxError


@dataclass
class SandboxAuth:
    control_url: str | None = None
    token: str | None = None


def resolve_auth(
    *,
    control_url: str | None = None,
    token: str | None = None,
) -> tuple[str, str | None]:
    url = control_url or os.environ.get("SANDBOX_CONTROL_URL")
    if not url:
        msg = "No control URL. Set SANDBOX_CONTROL_URL or pass control_url explicitly."
        raise SandboxError(msg)
    resolved_token = token or os.environ.get("SANDBOX_TOKEN")
    return url, resolved_token
