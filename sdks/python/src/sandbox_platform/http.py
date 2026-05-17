from __future__ import annotations

import asyncio
from typing import Any

import httpx

from sandbox_platform.errors import SandboxError

_RETRYABLE_STATUS_CODES = frozenset({429, 500, 502, 503, 504})
_MAX_RETRIES = 3
_BASE_DELAY_S = 1.0


def _raise_for_status(response: httpx.Response) -> None:
    if not response.is_success:
        raise SandboxError(f"HTTP {response.status_code}: {response.text}", response.status_code)


class HttpClient:
    def __init__(self, base_url: str, token: str | None = None) -> None:
        base = base_url.rstrip("/")
        headers: dict[str, str] = {}
        if token:
            headers["Authorization"] = f"Bearer {token}"
        self._client = httpx.AsyncClient(base_url=base, headers=headers)

    async def request(
        self,
        method: str,
        path: str,
        *,
        body: Any = None,
        params: dict[str, str] | None = None,
    ) -> Any:
        response = await self._request_with_retry(method, path, body=body, params=params)
        _raise_for_status(response)
        if response.status_code == 204:
            return None
        return response.json()

    async def request_raw(
        self,
        method: str,
        path: str,
        *,
        body: Any = None,
        params: dict[str, str] | None = None,
    ) -> httpx.Response:
        response = await self._request_with_retry(method, path, body=body, params=params)
        _raise_for_status(response)
        return response

    async def _request_with_retry(
        self,
        method: str,
        path: str,
        *,
        body: Any = None,
        params: dict[str, str] | None = None,
    ) -> httpx.Response:
        last_error: Exception | None = None
        kwargs: dict[str, Any] = {}
        if body is not None:
            kwargs["json"] = body
        if params is not None:
            kwargs["params"] = params

        for attempt in range(_MAX_RETRIES + 1):
            try:
                response = await self._client.request(method, path, **kwargs)

                if response.is_success or response.status_code not in _RETRYABLE_STATUS_CODES:
                    return response

                await response.aclose()

                if attempt < _MAX_RETRIES:
                    delay = _BASE_DELAY_S * (2**attempt)
                    await asyncio.sleep(delay)
                    continue

                return response
            except httpx.HTTPError as exc:
                last_error = exc
                if attempt < _MAX_RETRIES:
                    delay = _BASE_DELAY_S * (2**attempt)
                    await asyncio.sleep(delay)
                    continue

        raise last_error or SandboxError("request failed after retries")

    async def close(self) -> None:
        await self._client.aclose()
