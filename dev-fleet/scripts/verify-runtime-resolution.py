#!/usr/bin/env python3
"""Verify runtime-only pub API resolution on live fleet."""
from __future__ import annotations

import json
import sys
import time
import urllib.error
import urllib.request

CONTROLLER = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080"
EXPECTED_PREFIX = sys.argv[2] if len(sys.argv) > 2 else "localhost:5000"
TENANT = "verify-runtime-only"


def req(method: str, path: str, body: dict | None = None) -> tuple[int, dict]:
    data = json.dumps(body).encode() if body else None
    headers = {"X-Auth-Tenant": TENANT}
    if body:
        headers["Content-Type"] = "application/json"
    request = urllib.request.Request(
        f"{CONTROLLER}{path}", data=data, headers=headers, method=method
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as resp:
            raw = resp.read()
            if not raw:
                return resp.status, {}
            return resp.status, json.loads(raw)
    except urllib.error.HTTPError as e:
        raw = e.read()
        if not raw:
            return e.code, {}
        return e.code, json.loads(raw)


def main() -> int:
    print(f"Controller: {CONTROLLER}")
    print(f"Expected image prefix: {EXPECTED_PREFIX}")

    code, health = req("GET", "/pub/health")
    if code != 200 or health.get("status") != "ok":
        print(f"FAIL: controller unhealthy ({code} {health})")
        return 1
    print("PASS: controller healthy")

    code, created = req(
        "POST",
        "/pub/v1/sandboxes",
        {"runtime": "node24", "resources": {"vcpus": 1}, "ports": [8080]},
    )
    if code not in (200, 201):
        print(f"FAIL: create HTTP {code} {created}")
        return 1

    sid = created["sandbox"]["id"]
    image = created["sandbox"]["image"]
    print(f"  resolved image: {image}")

    if not image.startswith(EXPECTED_PREFIX):
        print(f"FAIL: image does not start with {EXPECTED_PREFIX!r}")
        req("DELETE", f"/pub/v1/sandboxes/{sid}")
        return 1
    if "sandbox-runtime:node24-latest" not in image:
        print("FAIL: image is not sandbox-runtime:node24-latest catalog entry")
        req("DELETE", f"/pub/v1/sandboxes/{sid}")
        return 1
    print("PASS: catalog resolution")

    for i in range(45):
        _, status_resp = req("GET", f"/pub/v1/sandboxes/{sid}")
        st = status_resp.get("sandbox", status_resp).get("status")
        if st == "ready":
            print(f"PASS: sandbox ready ({i * 2}s)")
            req("DELETE", f"/pub/v1/sandboxes/{sid}")
            return 0
        if st in ("terminated", "failed"):
            print(f"FAIL: sandbox {st}")
            req("DELETE", f"/pub/v1/sandboxes/{sid}")
            return 1
        time.sleep(2)

    print("FAIL: timeout waiting for ready")
    req("DELETE", f"/pub/v1/sandboxes/{sid}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
