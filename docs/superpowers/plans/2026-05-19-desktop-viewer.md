# Desktop Screen Viewer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a live, interactive remote desktop (via noVNC) when viewing a desktop-type sandbox in the console, with a collapsible terminal drawer underneath.

**Architecture:** Four layers — boot panel injects VNC env vars, agentd relays WebSocket to local websockify, ws-relay adds a `/vnc` endpoint mirroring the terminal pattern, and the console renders a `DesktopView` component with noVNC for desktop sandboxes.

**Tech Stack:** Rust/Axum (agentd WebSocket relay), Node/ws (ws-relay), React/Next.js + `@novnc/novnc` (console UI)

**Spec:** `docs/superpowers/specs/2026-05-19-desktop-viewer-design.md`

---

## File Structure

### New files
| File | Responsibility |
|---|---|
| `contracts/fixtures/agentd/vnc-unavailable.json` | Fixture: 503 response when VNC/websockify not running |
| `crates/agentd/src/handlers/vnc.rs` | WebSocket handler: upgrade, connect to local websockify, relay frames |
| `crates/agentd/tests/vnc.rs` | Contract test (Layer 2) + integration test for WebSocket relay |
| `console/src/components/desktop-view.tsx` | noVNC wrapper component — connects to ws-relay, renders canvas, manages connection lifecycle |
| `console/src/components/terminal-drawer.tsx` | Collapsible bottom drawer wrapping `TerminalView` — drag-resize, collapse/expand, session-persisted height |
| `console/__tests__/components/desktop-view.test.tsx` | Tests for DesktopView connection states and rendering |
| `console/__tests__/components/terminal-drawer.test.tsx` | Tests for drawer collapse/expand/resize behavior |

### Modified files
| File | Change |
|---|---|
| `docs/sandbox-platform-interfaces.md` | Add `GET (WS) /v1/vnc` to §10.1 endpoint summary + new §10.9 documenting the endpoint |
| `crates/agentd/src/handlers/mod.rs` | Add `pub mod vnc;` |
| `crates/agentd/src/server.rs` | Add `.route("/v1/vnc", get(handlers::vnc::stream))` |
| `crates/agentd/Cargo.toml` | Move `tokio-tungstenite` and `futures-util` to `[dependencies]` |
| `console/src/components/boot-sandbox-panel.tsx` | Auto-inject `VNC_ENABLED=1`, `VNC_VIEWONLY=0` for desktop runtime |
| `console/src/app/sandboxes/[id]/page.tsx` | Runtime-aware rendering: `DesktopView` + drawer for desktop, `TerminalView` for CLI |
| `console/ws-relay.ts` | Add `/vnc` WebSocket endpoint with binary frame relay |
| `console/package.json` | Add `@novnc/novnc` dependency |
| `console/__tests__/components/boot-sandbox-panel.test.tsx` | Add test: desktop runtime injects VNC env vars |

---

## Task 0: Propagation — interfaces.md + fixture

The propagation rule requires: interfaces.md → type → schema → fixture → test → code. The `/v1/vnc` endpoint is a WebSocket upgrade with no request/response body types (binary frame relay), so steps 2-4 (Rust type, schema, SDK types) are N/A. But the endpoint and its 503 error response must be documented and fixtured before any code.

**Files:**
- Modify: `docs/sandbox-platform-interfaces.md`
- Create: `contracts/fixtures/agentd/vnc-unavailable.json`

### Step 0.1: Update interfaces.md §10.1 endpoint summary

- [ ] In `docs/sandbox-platform-interfaces.md`, add the VNC endpoint to the §10.1 table. After the `/v1/computer` row, add:

```markdown
| GET (WS) | `/v1/vnc` | VNC WebSocket relay (desktop variant only) |
```

### Step 0.2: Add §10.9 documenting the VNC endpoint

- [ ] After §10.8 (`POST /v1/computer`), add:

```markdown
### 10.9 `GET /v1/vnc` (WS, desktop variant)

WebSocket upgrade. Relays binary VNC/RFB frames between the caller and local websockify (`ws://127.0.0.1:6080/websockify`). No protocol interpretation — agentd is a transparent byte relay.

**Upgrade:** Standard WebSocket handshake. On success, binary frames flow bidirectionally. Either side closing terminates both connections.

**503 when websockify not running:**

```ts
// HTTP 503 Service Unavailable
type VncUnavailableResponse = {
  error: "vnc_not_available";
  message: string;
}
```

This doubles as a VNC readiness probe. The console retries with backoff until the upgrade succeeds.

**Note:** Only meaningful on desktop sandboxes where `VNC_ENABLED=1`. Non-desktop sandboxes have no websockify process; the endpoint will always return 503.
```

### Step 0.3: Create the 503 fixture

- [ ] Create `contracts/fixtures/agentd/vnc-unavailable.json`:

```json
{
  "error": "vnc_not_available",
  "message": "VNC is not enabled on this sandbox"
}
```

### Step 0.4: Commit

```bash
git add docs/sandbox-platform-interfaces.md \
  contracts/fixtures/agentd/vnc-unavailable.json
git commit -m "$(cat <<'EOF'
contracts: docs: add /v1/vnc endpoint to interfaces.md and fixture

Propagation rule: document the endpoint and its 503 response shape
before writing any code. The endpoint is a WebSocket upgrade that
relays binary VNC frames to local websockify. No request/response
body types needed (opaque binary relay), so schema/SDK steps are N/A.
EOF
)"
```

---

## Task 1: agentd `/v1/vnc` WebSocket relay

**Files:**
- Create: `crates/agentd/src/handlers/vnc.rs`
- Modify: `crates/agentd/src/handlers/mod.rs`
- Modify: `crates/agentd/src/server.rs`
- Modify: `crates/agentd/Cargo.toml`
- Test: `crates/agentd/tests/vnc.rs`

### Step 1.1: Write failing contract test — 503 matches fixture

- [ ] Create `crates/agentd/tests/vnc.rs`:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::test_helpers::load_fixture;
use reqwest::StatusCode;

#[tokio::test]
async fn contract_vnc_unavailable_matches_fixture() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/vnc");
    let resp = reqwest::get(&url).await.expect("request failed");
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body: serde_json::Value = resp.json().await.expect("json parse failed");
    let fixture: serde_json::Value = load_fixture("agentd/vnc-unavailable.json");
    assert_eq!(body, fixture);
}
```

- [ ] Run to verify it fails:

Run: `cargo test -p agentd --test vnc -- contract_vnc_unavailable 2>&1 | tail -20`
Expected: compilation error — no handler exists yet.

### Step 1.2: Commit the red test

```bash
git add crates/agentd/tests/vnc.rs
git commit -m "$(cat <<'EOF'
agentd: red: failing contract test for /v1/vnc 503 response

Loads the vnc-unavailable.json fixture and asserts the endpoint
returns an exact match. Fails because the endpoint does not exist.
EOF
)"
```

### Step 1.3: Implement the 503 path + route registration

- [ ] Create `crates/agentd/src/handlers/vnc.rs`:

```rust
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{client_async, tungstenite};

const WEBSOCKIFY_ADDR: &str = "127.0.0.1:6080";

pub async fn stream(ws: WebSocketUpgrade) -> impl IntoResponse {
    match TcpStream::connect(WEBSOCKIFY_ADDR).await {
        Ok(probe) => {
            drop(probe);
            ws.on_upgrade(relay).into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": "vnc_not_available",
                "message": "VNC is not enabled on this sandbox"
            })),
        )
            .into_response(),
    }
}

async fn relay(mut client: WebSocket) {
    let tcp = match TcpStream::connect(WEBSOCKIFY_ADDR).await {
        Ok(s) => s,
        Err(_) => return,
    };

    let ws_url = format!("ws://{WEBSOCKIFY_ADDR}/websockify");
    let (mut upstream, _) = match client_async(&ws_url, tcp).await {
        Ok(pair) => pair,
        Err(_) => return,
    };

    loop {
        tokio::select! {
            msg = StreamExt::next(&mut upstream) => {
                match msg {
                    Some(Ok(tungstenite::Message::Binary(data))) => {
                        if client.send(Message::Binary(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(tungstenite::Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            msg = client.recv() => {
                match msg {
                    Some(Ok(Message::Binary(data))) => {
                        if upstream.send(tungstenite::Message::Binary(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    let _ = upstream.close(None).await;
}
```

- [ ] Add `pub mod vnc;` to `crates/agentd/src/handlers/mod.rs` after the last existing module.

- [ ] In `crates/agentd/src/server.rs`, add `.route("/v1/vnc", get(handlers::vnc::stream))` after the `/v1/computer` line.

- [ ] Move `tokio-tungstenite` and `futures-util` from `[dev-dependencies]` to `[dependencies]` in `crates/agentd/Cargo.toml`:

```toml
[dependencies]
# ... existing deps ...
tokio-tungstenite = "0.26"
futures-util = "0.3"
```

Remove the duplicate entries from `[dev-dependencies]`.

- [ ] Run contract test:

Run: `cargo test -p agentd --test vnc -- contract_vnc_unavailable 2>&1 | tail -10`
Expected: PASS — no websockify is listening on 6080 in the test environment, so 503 is returned matching the fixture.

### Step 1.4: Commit the green implementation

```bash
git add crates/agentd/src/handlers/vnc.rs crates/agentd/src/handlers/mod.rs \
  crates/agentd/src/server.rs crates/agentd/Cargo.toml
git commit -m "$(cat <<'EOF'
agentd: green: implement /v1/vnc WebSocket relay endpoint

Relays binary WebSocket frames between the caller (Plane proxy)
and local websockify on port 6080. Returns 503 matching the
vnc-unavailable fixture if websockify is not running.
EOF
)"
```

### Step 1.5: Write integration test — WebSocket relay with mock websockify

- [ ] Add to `crates/agentd/tests/vnc.rs`:

```rust
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn vnc_relays_binary_frames() {
    let listener = TcpListener::bind("127.0.0.1:6080").await.expect("bind 6080");

    let mock_handle = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept");
        let mut ws = accept_async(stream).await.expect("ws accept");
        while let Some(Ok(msg)) = ws.next().await {
            if let Message::Binary(data) = msg {
                let mut reply = b"echo:".to_vec();
                reply.extend_from_slice(&data);
                if ws.send(Message::Binary(reply.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let addr = agentd::serve_on_ephemeral().await;
    let ws_url = format!("ws://{addr}/v1/vnc");

    let (mut client, _) = tokio_tungstenite::connect_async(&ws_url)
        .await
        .expect("connect to agentd /v1/vnc");

    client
        .send(Message::Binary(b"hello".to_vec().into()))
        .await
        .expect("send");

    let reply = client.next().await.expect("reply").expect("reply ok");
    match reply {
        Message::Binary(data) => assert_eq!(&data[..], b"echo:hello"),
        other => panic!("expected binary, got {other:?}"),
    }

    client.close(None).await.expect("close");
    mock_handle.abort();
}
```

- [ ] Run: `cargo test -p agentd --test vnc 2>&1 | tail -10`
Expected: both tests PASS.

### Step 1.6: Commit

```bash
git add crates/agentd/tests/vnc.rs
git commit -m "$(cat <<'EOF'
agentd: green: add integration test for VNC WebSocket relay

Spins up a mock websockify on port 6080 and verifies agentd
relays binary frames bidirectionally through /v1/vnc.
EOF
)"
```

---

## Task 2: ws-relay `/vnc` endpoint

**Files:**
- Modify: `console/ws-relay.ts`

### Step 2.1: Add `/vnc` path handler to the upgrade event

- [ ] In `console/ws-relay.ts`, add a new branch in the `server.on("upgrade", ...)` callback, after the existing `/terminal` branch:

```typescript
  } else if (url.pathname === "/vnc") {
    wss.handleUpgrade(req, socket, head, (ws) => {
      handleVncRelay(ws, url.searchParams);
    });
  } else {
```

### Step 2.2: Implement handleVncRelay function

- [ ] Add this function after `handleTerminalRelay` in `console/ws-relay.ts`:

```typescript
async function handleVncRelay(clientWs: WebSocket, params: URLSearchParams) {
  const sandboxId = params.get("sandbox");

  if (!sandboxId) {
    clientWs.close(1008, "missing sandbox param");
    return;
  }

  let proxyUrl: string;
  try {
    proxyUrl = await resolveProxyUrl(sandboxId);
  } catch (err) {
    const msg = err instanceof Error ? err.message : "failed to resolve sandbox";
    clientWs.close(1011, msg);
    return;
  }

  const upstreamUrl = `${proxyUrl.replace(/^http/, "ws")}/v1/vnc`;
  const upstreamWs = new WebSocket(upstreamUrl);
  upstreamWs.binaryType = "arraybuffer";

  upstreamWs.on("open", () => {
    console.log(`[ws-relay] vnc connected: sandbox=${sandboxId}`);
  });

  upstreamWs.on("message", (data: Buffer | ArrayBuffer, isBinary: boolean) => {
    if (clientWs.readyState !== WebSocket.OPEN) return;
    clientWs.send(data, { binary: isBinary });
  });

  clientWs.on("message", (data: Buffer | ArrayBuffer, isBinary: boolean) => {
    if (upstreamWs.readyState !== WebSocket.OPEN) return;
    upstreamWs.send(data, { binary: isBinary });
  });

  clientWs.on("close", () => {
    upstreamWs.close();
  });

  upstreamWs.on("close", (code: number, reason: Buffer) => {
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.close(code, reason.toString());
    }
  });

  upstreamWs.on("error", (err: Error) => {
    console.error(`[ws-relay] vnc upstream error: sandbox=${sandboxId}:`, err.message);
    if (clientWs.readyState === WebSocket.OPEN) {
      clientWs.close(1011, `upstream: ${err.message}`);
    }
  });

  clientWs.on("error", (err: Error) => {
    console.error(`[ws-relay] vnc client error: sandbox=${sandboxId}:`, err.message);
    upstreamWs.close();
  });
}
```

### Step 2.3: Verify TypeScript compiles

- [ ] Run: `cd console && npx tsc --noEmit 2>&1 | tail -10`
Expected: no errors.

### Step 2.4: Commit

```bash
git add console/ws-relay.ts
git commit -m "$(cat <<'EOF'
console: feat: add /vnc WebSocket relay endpoint to ws-relay

Mirrors the terminal relay pattern. Resolves sandbox connection URL,
connects to {proxyUrl}/v1/vnc, relays binary frames bidirectionally.
No VNC protocol interpretation — transparent byte relay.
EOF
)"
```

---

## Task 3: Boot panel auto-injects VNC env vars

**Files:**
- Modify: `console/src/components/boot-sandbox-panel.tsx`
- Test: `console/__tests__/components/boot-sandbox-panel.test.tsx`

### Step 3.1: Write failing test — desktop runtime includes VNC env vars

- [ ] Add to `console/__tests__/components/boot-sandbox-panel.test.tsx`, inside the `describe` block:

```typescript
  it("injects VNC env vars when booting desktop runtime", async () => {
    const user = userEvent.setup();
    renderPanel();

    await user.click(screen.getByRole("button", { name: /Desktop/i }));

    const callArgs = mockMutate.mock.calls[0][0];
    expect(callArgs.env).toBeDefined();
    expect(callArgs.env.VNC_ENABLED).toBe("1");
    expect(callArgs.env.VNC_VIEWONLY).toBe("0");
  });

  it("does not inject VNC env vars for non-desktop runtimes", async () => {
    const user = userEvent.setup();
    renderPanel();

    await user.click(screen.getByRole("button", { name: /Node.js 24/i }));

    const callArgs = mockMutate.mock.calls[0][0];
    expect(callArgs.env?.VNC_ENABLED).toBeUndefined();
  });
```

- [ ] Run to verify failure:

Run: `cd console && npx vitest run __tests__/components/boot-sandbox-panel.test.tsx 2>&1 | tail -15`
Expected: FAIL — VNC env vars not injected yet.

### Step 3.2: Commit the red test

```bash
git add console/__tests__/components/boot-sandbox-panel.test.tsx
git commit -m "$(cat <<'EOF'
console: red: failing tests for VNC env var injection in boot panel

Desktop runtime should auto-inject VNC_ENABLED=1 and VNC_VIEWONLY=0.
Non-desktop runtimes should not have VNC env vars.
EOF
)"
```

### Step 3.3: Implement VNC env var injection

- [ ] In `console/src/components/boot-sandbox-panel.tsx`, modify the `handleBoot` callback. Replace the env construction inside `createSandbox.mutate(`:

Change:
```typescript
          ...(envKeys.length > 0 ? { env: config.env } : {}),
```

To:
```typescript
          ...(() => {
            const env = { ...config.env };
            if (preset.runtime === "desktop") {
              env.VNC_ENABLED = "1";
              env.VNC_VIEWONLY = "0";
            }
            return Object.keys(env).length > 0 ? { env } : {};
          })(),
```

- [ ] Run tests:

Run: `cd console && npx vitest run __tests__/components/boot-sandbox-panel.test.tsx 2>&1 | tail -15`
Expected: all tests PASS.

### Step 3.4: Commit the green implementation

```bash
git add console/src/components/boot-sandbox-panel.tsx
git commit -m "$(cat <<'EOF'
console: green: auto-inject VNC_ENABLED=1 for desktop sandboxes

When the boot panel creates a desktop sandbox, it merges VNC_ENABLED=1
and VNC_VIEWONLY=0 into the environment. No UI change.
EOF
)"
```

---

## Task 4: Install `@novnc/novnc` dependency

**Files:**
- Modify: `console/package.json`

### Step 4.1: Install the package

- [ ] Run: `cd console && npm install @novnc/novnc`

- [ ] Verify it's in `package.json` dependencies:

Run: `grep novnc console/package.json`
Expected: `"@novnc/novnc": "^X.Y.Z"` in dependencies.

### Step 4.2: Commit

```bash
git add console/package.json console/package-lock.json
git commit -m "$(cat <<'EOF'
console: chore: add @novnc/novnc dependency

MIT-licensed noVNC client library for the desktop viewer component.
EOF
)"
```

---

## Task 5: `DesktopView` component

**Files:**
- Create: `console/src/components/desktop-view.tsx`
- Test: `console/__tests__/components/desktop-view.test.tsx`

### Step 5.1: Write failing test — component renders with connection states

- [ ] Create `console/__tests__/components/desktop-view.test.tsx`:

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";

vi.mock("@novnc/novnc/core/rfb.js", () => {
  return {
    default: vi.fn().mockImplementation(() => ({
      scaleViewport: false,
      resizeSession: false,
      focusOnClick: false,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      disconnect: vi.fn(),
      get _target() {
        return document.createElement("div");
      },
    })),
  };
});

import { DesktopView } from "@/components/desktop-view";

describe("DesktopView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders the VNC toolbar with connecting state", () => {
    render(<DesktopView sandboxId="sb-test-1" />);
    expect(screen.getByText(/connecting/i)).toBeDefined();
  });

  it("calls onConnectionChange when state changes", () => {
    const onChange = vi.fn();
    render(<DesktopView sandboxId="sb-test-1" onConnectionChange={onChange} />);
    expect(onChange).toHaveBeenCalledWith("connecting");
  });

  it("renders fullscreen button", () => {
    render(<DesktopView sandboxId="sb-test-1" />);
    expect(screen.getByRole("button", { name: /fullscreen/i })).toBeDefined();
  });
});
```

- [ ] Run to verify failure:

Run: `cd console && npx vitest run __tests__/components/desktop-view.test.tsx 2>&1 | tail -15`
Expected: FAIL — module not found.

### Step 5.2: Commit the red test

```bash
git add console/__tests__/components/desktop-view.test.tsx
git commit -m "$(cat <<'EOF'
console: red: failing tests for DesktopView component

Tests connection state rendering, onConnectionChange callback,
and fullscreen button. Fails because component doesn't exist yet.
EOF
)"
```

### Step 5.3: Implement DesktopView

- [ ] Create `console/src/components/desktop-view.tsx`:

```tsx
"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import { Maximize, Minimize } from "lucide-react";

type ConnectionState = "connecting" | "connected" | "disconnected";

type DesktopViewProps = {
  sandboxId: string;
  onConnectionChange?: (state: ConnectionState) => void;
};

const WS_RELAY_URL =
  process.env.NEXT_PUBLIC_WS_RELAY_URL ?? "ws://localhost:3101";

export function DesktopView({ sandboxId, onConnectionChange }: DesktopViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const rfbRef = useRef<InstanceType<typeof import("@novnc/novnc/core/rfb.js").default> | null>(null);
  const [connState, setConnState] = useState<ConnectionState>("connecting");
  const [resolution, setResolution] = useState<string>("");
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [retryIn, setRetryIn] = useState(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const retryDelayRef = useRef(1000);
  const onConnectionChangeRef = useRef(onConnectionChange);
  onConnectionChangeRef.current = onConnectionChange;

  const updateState = useCallback((state: ConnectionState) => {
    setConnState(state);
    onConnectionChangeRef.current?.(state);
  }, []);

  const connect = useCallback(async () => {
    if (!containerRef.current) return;

    updateState("connecting");
    retryDelayRef.current = 1000;

    const { default: RFB } = await import("@novnc/novnc/core/rfb.js");

    const url = `${WS_RELAY_URL}/vnc?sandbox=${encodeURIComponent(sandboxId)}`;
    const rfb = new RFB(containerRef.current, url);
    rfb.scaleViewport = true;
    rfb.resizeSession = false;
    rfb.focusOnClick = true;
    rfbRef.current = rfb;

    rfb.addEventListener("connect", () => {
      updateState("connected");
      retryDelayRef.current = 1000;
      const canvas = containerRef.current?.querySelector("canvas");
      if (canvas) {
        setResolution(`${canvas.width}×${canvas.height}`);
      }
    });

    rfb.addEventListener("disconnect", (e: CustomEvent) => {
      rfbRef.current = null;
      updateState("disconnected");
      if (!e.detail.clean) {
        scheduleRetry();
      }
    });
  }, [sandboxId, updateState]);

  const scheduleRetry = useCallback(() => {
    const delay = retryDelayRef.current;
    retryDelayRef.current = Math.min(delay * 2, 10000);

    let remaining = Math.ceil(delay / 1000);
    setRetryIn(remaining);

    const tick = setInterval(() => {
      remaining -= 1;
      setRetryIn(remaining);
      if (remaining <= 0) clearInterval(tick);
    }, 1000);

    retryTimerRef.current = setTimeout(() => {
      clearInterval(tick);
      setRetryIn(0);
      connect();
    }, delay);
  }, [connect]);

  useEffect(() => {
    connect();
    return () => {
      if (retryTimerRef.current) clearTimeout(retryTimerRef.current);
      rfbRef.current?.disconnect();
      rfbRef.current = null;
    };
  }, [connect]);

  useEffect(() => {
    const handler = () => setIsFullscreen(!!document.fullscreenElement);
    document.addEventListener("fullscreenchange", handler);
    return () => document.removeEventListener("fullscreenchange", handler);
  }, []);

  const toggleFullscreen = useCallback(() => {
    const el = containerRef.current?.parentElement;
    if (!el) return;
    if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      el.requestFullscreen();
    }
  }, []);

  const statusDot = {
    connecting: "bg-status-amber",
    connected: "bg-status-green",
    disconnected: "bg-status-red",
  }[connState];

  const statusText = connState === "connected"
    ? `vnc · ${resolution}`
    : connState === "connecting"
      ? "connecting..."
      : retryIn > 0
        ? `disconnected — retry in ${retryIn}s`
        : "disconnected";

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-4 py-1.5 border-b border-border bg-bg-1">
        <div className="flex items-center gap-2">
          <span className={`w-1.5 h-1.5 rounded-full ${statusDot}`} />
          <span className="text-xs text-text-3 font-mono">{statusText}</span>
        </div>
        <button
          type="button"
          onClick={toggleFullscreen}
          className="flex items-center gap-1.5 text-xs text-text-3 hover:text-text transition-colors"
          aria-label={isFullscreen ? "Exit fullscreen" : "Fullscreen"}
        >
          {isFullscreen ? <Minimize size={12} /> : <Maximize size={12} />}
          <span className="hidden sm:inline">{isFullscreen ? "Exit" : "Fullscreen"}</span>
        </button>
      </div>
      <div ref={containerRef} className="flex-1 min-h-0 bg-black" />
    </div>
  );
}
```

- [ ] Run tests:

Run: `cd console && npx vitest run __tests__/components/desktop-view.test.tsx 2>&1 | tail -15`
Expected: all tests PASS.

### Step 5.4: Commit the green implementation

```bash
git add console/src/components/desktop-view.tsx
git commit -m "$(cat <<'EOF'
console: green: implement DesktopView component

Wraps @novnc/novnc RFB client. Connects to ws-relay /vnc endpoint,
scale-to-fit, auto-retry with exponential backoff (1s-10s cap).
Toolbar shows connection status dot and fullscreen toggle.
EOF
)"
```

---

## Task 6: `TerminalDrawer` component

**Files:**
- Create: `console/src/components/terminal-drawer.tsx`
- Test: `console/__tests__/components/terminal-drawer.test.tsx`

### Step 6.1: Write failing test

- [ ] Create `console/__tests__/components/terminal-drawer.test.tsx`:

```typescript
import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("@/components/terminal-view", () => ({
  TerminalView: ({ sandboxId }: { sandboxId: string }) => (
    <div data-testid="terminal-view">terminal:{sandboxId}</div>
  ),
}));

import { TerminalDrawer } from "@/components/terminal-drawer";

describe("TerminalDrawer", () => {
  it("renders collapsed by default with Terminal label", () => {
    render(<TerminalDrawer sandboxId="sb-test-1" />);
    expect(screen.getByText("Terminal")).toBeDefined();
    expect(screen.queryByTestId("terminal-view")).toBeNull();
  });

  it("expands on click to show TerminalView", async () => {
    const user = userEvent.setup();
    render(<TerminalDrawer sandboxId="sb-test-1" />);

    await user.click(screen.getByText("Terminal"));
    expect(screen.getByTestId("terminal-view")).toBeDefined();
  });

  it("collapses when clicking the collapse button", async () => {
    const user = userEvent.setup();
    render(<TerminalDrawer sandboxId="sb-test-1" />);

    await user.click(screen.getByText("Terminal"));
    expect(screen.getByTestId("terminal-view")).toBeDefined();

    await user.click(screen.getByRole("button", { name: /collapse/i }));
    expect(screen.queryByTestId("terminal-view")).toBeNull();
  });
});
```

- [ ] Run to verify failure:

Run: `cd console && npx vitest run __tests__/components/terminal-drawer.test.tsx 2>&1 | tail -15`
Expected: FAIL — module not found.

### Step 6.2: Commit the red test

```bash
git add console/__tests__/components/terminal-drawer.test.tsx
git commit -m "$(cat <<'EOF'
console: red: failing tests for TerminalDrawer component

Tests collapsed default state, expand on click, collapse button.
Fails because component doesn't exist yet.
EOF
)"
```

### Step 6.3: Implement TerminalDrawer

- [ ] Create `console/src/components/terminal-drawer.tsx`:

```tsx
"use client";

import { useState, useRef, useCallback, useEffect } from "react";
import { ChevronUp, ChevronDown } from "lucide-react";
import { TerminalView } from "@/components/terminal-view";
import type { ExecEntry } from "@/api/types";

const STORAGE_KEY = "terminal-drawer-height";
const DEFAULT_HEIGHT = 200;
const MIN_HEIGHT = 100;
const COLLAPSED_HEIGHT = 32;

type TerminalDrawerProps = {
  sandboxId: string;
  onCommand?: (cmd: string) => void;
  onExecComplete?: (entry: ExecEntry) => void;
};

export function TerminalDrawer({ sandboxId, onCommand, onExecComplete }: TerminalDrawerProps) {
  const [expanded, setExpanded] = useState(false);
  const [height, setHeight] = useState(() => {
    if (typeof window === "undefined") return DEFAULT_HEIGHT;
    const stored = sessionStorage.getItem(STORAGE_KEY);
    return stored ? parseInt(stored, 10) : DEFAULT_HEIGHT;
  });
  const dragging = useRef(false);
  const startY = useRef(0);
  const startHeight = useRef(0);

  useEffect(() => {
    if (expanded) {
      sessionStorage.setItem(STORAGE_KEY, String(height));
    }
  }, [height, expanded]);

  const onDragStart = useCallback(
    (e: React.MouseEvent) => {
      if (!expanded) return;
      e.preventDefault();
      dragging.current = true;
      startY.current = e.clientY;
      startHeight.current = height;

      const onMove = (ev: MouseEvent) => {
        if (!dragging.current) return;
        const delta = startY.current - ev.clientY;
        const maxHeight = window.innerHeight * 0.6;
        const next = Math.max(MIN_HEIGHT, Math.min(maxHeight, startHeight.current + delta));
        setHeight(next);
      };

      const onUp = () => {
        dragging.current = false;
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      };

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    },
    [expanded, height],
  );

  const toggle = useCallback(() => setExpanded((prev) => !prev), []);

  return (
    <div style={{ height: expanded ? height + COLLAPSED_HEIGHT : COLLAPSED_HEIGHT }}
         className="shrink-0 border-t-2 border-border-light flex flex-col">
      <div
        className={`flex items-center justify-between px-4 shrink-0 select-none ${
          expanded ? "cursor-row-resize" : "cursor-pointer"
        } bg-bg-1 hover:bg-bg-hover transition-colors`}
        style={{ height: COLLAPSED_HEIGHT }}
        onClick={expanded ? undefined : toggle}
        onMouseDown={expanded ? onDragStart : undefined}
        role="separator"
        aria-orientation="horizontal"
      >
        <div className="flex items-center gap-2">
          {expanded ? <ChevronDown size={12} className="text-text-3" /> : <ChevronUp size={12} className="text-text-3" />}
          <span className="text-xs text-text-3 font-mono">Terminal</span>
        </div>
        {expanded ? (
          <button
            type="button"
            onClick={(e) => { e.stopPropagation(); toggle(); }}
            className="text-xs text-text-3 hover:text-text transition-colors"
            aria-label="Collapse terminal"
          >
            collapse
          </button>
        ) : (
          <span className="text-xs text-text-4">click to expand</span>
        )}
      </div>
      {expanded && (
        <div className="flex-1 min-h-0 overflow-hidden">
          <TerminalView sandboxId={sandboxId} onCommand={onCommand} onExecComplete={onExecComplete} />
        </div>
      )}
    </div>
  );
}
```

- [ ] Run tests:

Run: `cd console && npx vitest run __tests__/components/terminal-drawer.test.tsx 2>&1 | tail -15`
Expected: all tests PASS.

### Step 6.4: Commit the green implementation

```bash
git add console/src/components/terminal-drawer.tsx
git commit -m "$(cat <<'EOF'
console: green: implement TerminalDrawer component

Collapsible bottom drawer wrapping TerminalView. Default collapsed
(32px bar). Click to expand, drag to resize, max 60% viewport.
Height persisted in sessionStorage.
EOF
)"
```

---

## Task 7: Sandbox detail page — runtime-aware rendering

**Files:**
- Modify: `console/src/app/sandboxes/[id]/page.tsx`

### Step 7.1: Import new components and add desktop detection

- [ ] Add imports at the top of `console/src/app/sandboxes/[id]/page.tsx`:

```typescript
import { DesktopView } from "@/components/desktop-view";
import { TerminalDrawer } from "@/components/terminal-drawer";
```

### Step 7.2: Add desktop runtime check

- [ ] After the `isBooting` declaration (line ~54 area), add:

```typescript
const isDesktop = sandbox.runtime === "desktop";
```

### Step 7.3: Replace the main content area rendering

- [ ] Replace the entire conditional rendering chain inside the main content `<div>` (the block that starts with `{isStopping ? (` through to the matching closing `)}`) with:

```tsx
          {isStopping ? (
            <div className="flex flex-col items-center justify-center h-full gap-3">
              <div className="w-2 h-2 rounded-full bg-status-red animate-pulse" />
              <p className="text-sm text-text-2">Stopping sandbox…</p>
            </div>
          ) : isLive && isDesktop ? (
            <div className="flex flex-col h-full">
              <DesktopView sandboxId={id} />
              <TerminalDrawer
                sandboxId={id}
                onCommand={(cmd) => { lastCommandRef.current = cmd; }}
                onExecComplete={(entry) => { setExecHistory((prev) => [...prev, entry]); }}
              />
            </div>
          ) : isLive ? (
            <TerminalView
              sandboxId={id}
              onCommand={(cmd) => { lastCommandRef.current = cmd; }}
              onExecComplete={(entry) => { setExecHistory((prev) => [...prev, entry]); }}
            />
          ) : isBooting ? (
            <div className="flex flex-col items-center justify-center h-full gap-3">
              <p className="text-sm text-text-2">Sandbox is {sandbox.status}…</p>
              <ColdStartTimer running={true} />
            </div>
          ) : (
            <div className="flex items-center justify-center h-full text-text-3 text-sm">
              Sandbox is {sandbox.status}
            </div>
          )}
```

### Step 7.4: Verify build and existing tests

- [ ] Run: `cd console && npx tsc --noEmit 2>&1 | tail -10`
Expected: no type errors.

- [ ] Run: `cd console && npx vitest run 2>&1 | tail -20`
Expected: all existing tests still pass.

### Step 7.5: Commit

```bash
git add console/src/app/sandboxes/[id]/page.tsx
git commit -m "$(cat <<'EOF'
console: feat: render DesktopView for desktop sandboxes

When sandbox.runtime is 'desktop', the detail page shows the noVNC
desktop viewer with a collapsible terminal drawer underneath.
CLI runtimes continue to render TerminalView as before.
EOF
)"
```

---

## Task 8: Manual smoke test

No code changes — verification only.

### Step 8.1: Start the dev environment

- [ ] Run: `make console-dev`

### Step 8.2: Boot a desktop sandbox

- [ ] Open `http://localhost:3100` in the browser.
- [ ] Click the **Desktop** button on the quickstart page.
- [ ] Verify the sandbox boots and redirects to the detail page.

### Step 8.3: Verify desktop view

- [ ] Confirm the VNC toolbar appears with an amber "connecting..." status.
- [ ] If the dev fleet is running with a real desktop sandbox, confirm the status turns green and the desktop screen renders.
- [ ] If no fleet is available, confirm the status shows "disconnected" with retry countdown (expected — no real websockify to connect to).

### Step 8.4: Verify terminal drawer

- [ ] Confirm the collapsed "Terminal" bar appears at the bottom.
- [ ] Click it to expand. Verify the terminal renders.
- [ ] Drag the handle to resize.
- [ ] Click "collapse" to collapse.

### Step 8.5: Verify CLI sandboxes unaffected

- [ ] Boot a Node.js 24 sandbox.
- [ ] Confirm it renders the standard terminal view with no desktop viewer or drawer.

### Step 8.6: Verify VNC env var injection

- [ ] In the browser DevTools Network tab, find the `POST /api/sandboxes` request when booting a desktop sandbox.
- [ ] Confirm the request body contains `"env": { "VNC_ENABLED": "1", "VNC_VIEWONLY": "0", ... }`.
- [ ] Boot a Node.js sandbox and confirm the request body does NOT contain VNC env vars.

---

## Task Summary

| Task | Layer | Description |
|---|---|---|
| 0 | Contracts | Propagation: update interfaces.md §10 + add VNC 503 fixture |
| 1 | agentd (Rust) | `/v1/vnc` WebSocket relay — red → green with contract test |
| 2 | ws-relay (Node) | `/vnc` binary frame relay endpoint |
| 3 | Boot panel (React) | Auto-inject `VNC_ENABLED=1` — red → green |
| 4 | Dependencies | Install `@novnc/novnc` |
| 5 | DesktopView (React) | noVNC wrapper — red → green |
| 6 | TerminalDrawer (React) | Collapsible drawer — red → green |
| 7 | Detail page (React) | Runtime-aware rendering |
| 8 | Smoke test | Manual verification of all paths |

**Parallelism:** Task 0 must come first. Then Tasks 1-2 (backend) and Tasks 3-6 (frontend) are independent and can run in parallel. Task 7 depends on 5+6. Task 8 depends on all.

**Commit rhythm:** Each TDD cycle produces separate `red:` and `green:` commits per discipline §3.2. The session-ending commit should include `Phase:` and `Next:` trailers per discipline §4.1.
