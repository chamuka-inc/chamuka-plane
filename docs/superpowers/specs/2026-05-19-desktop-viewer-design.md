# Desktop Screen Viewer — Design Spec

**Date:** 2026-05-19
**Status:** Approved
**Scope:** Show the remote desktop GUI when viewing a desktop-type sandbox in the console

---

## Problem

The console treats all sandbox types identically — desktop sandboxes get the same xterm.js terminal as CLI sandboxes. The desktop runtime image already ships Xvfb + Openbox + x11vnc + noVNC (websockify on port 6080), but VNC is disabled by default and the console has no viewer component. Users who boot a desktop sandbox have no way to see or interact with the graphical environment from the console.

## Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Layout model | Desktop-first with collapsible terminal drawer | Desktop is the point; terminal is a power-user escape hatch |
| VNC connectivity | agentd WebSocket relay at `/v1/vnc` | Reuses existing auth path, no proxy changes, natural gateway |
| Interactivity | Full interactive (mouse + keyboard) | It's a desktop — users want to click around |
| VNC activation | Always-on for desktop runtime | If you picked Desktop, you want a desktop |
| Scaling | Scale-to-fit container | Clean default, matches standard remote desktop UX |

## Architecture

### Data flow

```
Browser (noVNC.js)
  → WebSocket → ws-relay :3101/vnc?sandbox={id}
    → WebSocket → Plane proxy /{token}/v1/vnc
      → agentd /v1/vnc
        → ws://localhost:6080/websockify (x11vnc + websockify)
          → Xvfb :99 (virtual framebuffer)
```

All hops are WebSocket. The VNC protocol (RFB) travels as binary WebSocket frames end-to-end. No hop interprets VNC — they're pure relays.

### Layer 1 — Desktop image (no changes)

The Dockerfile already installs Xvfb, Openbox, x11vnc, websockify, and noVNC. The `start.sh` script conditionally starts VNC when `VNC_ENABLED=1`.

The only change is at boot time: the console's boot panel injects `VNC_ENABLED=1` and `VNC_VIEWONLY=0` into the sandbox environment when the runtime is `desktop`.

### Layer 2 — agentd: `/v1/vnc` endpoint

New WebSocket endpoint on the agentd HTTP server.

**Request:** `GET /v1/vnc` with `Upgrade: websocket` header.

**Behavior:**
1. Accept the WebSocket upgrade from the caller (Plane proxy).
2. Connect to `ws://127.0.0.1:6080/websockify` (local websockify).
3. If websockify is not listening, respond `503 Service Unavailable` with body `{"error": "vnc_not_available", "message": "VNC is not enabled on this sandbox"}`.
4. Relay binary frames bidirectionally between the two WebSocket connections.
5. When either side closes, close the other.

**Why agentd and not a direct port route:** The Plane proxy routes `/{token}/*` to a single backend address (agentd on port 8080). Adding port-routing to the proxy would touch the fork's core routing layer. agentd is the in-sandbox gateway and already handles all inbound requests — adding a relay endpoint is ~50 lines and keeps the proxy unchanged.

**Health signal:** The `/v1/vnc` endpoint doubles as a VNC readiness probe. The console can attempt the WebSocket connection and handle 503 to show "Desktop starting..." instead of a connection error.

### Layer 3 — ws-relay: `/vnc` endpoint

New WebSocket endpoint on the existing ws-relay server (`console/ws-relay.ts`).

**Client connects:** `ws://localhost:3101/vnc?sandbox={sandboxId}`

**Behavior:**
1. Resolve the sandbox's connection URL via the BFF (`GET /api/sandboxes/{id}` → `connection.url`).
2. Connect to `{proxyUrl}/v1/vnc` as a WebSocket client.
3. Relay binary frames bidirectionally.
4. On upstream disconnect, send close frame to client with reason.

This mirrors the existing terminal relay pattern. The relay does not interpret VNC frames — it's a transparent WebSocket proxy adding the sandbox-resolution step.

### Layer 4 — Console UI

#### New component: `DesktopView`

**File:** `console/src/components/desktop-view.tsx`

**Dependencies:** `@novnc/novnc` npm package (MIT, the standard noVNC client library).

**Props:**
```typescript
type DesktopViewProps = {
  sandboxId: string;
  onConnectionChange?: (state: "connecting" | "connected" | "disconnected") => void;
};
```

**Behavior:**
- Creates a `RFB` instance from `@novnc/novnc` pointed at `${NEXT_PUBLIC_WS_RELAY_URL}/vnc?sandbox={sandboxId}` (same env var the terminal view uses).
- Sets `scaleViewport = true` (scale-to-fit), `resizeSession = false` (don't change remote resolution).
- Renders the noVNC canvas inside a container div that fills available space.
- On disconnect, auto-retries with exponential backoff (1s, 2s, 4s, capped at 10s).
- On unmount, cleanly disconnects.

**Keyboard handling:** When the noVNC canvas has focus, all keyboard events go to VNC (noVNC handles this natively via `focusOnClick`). Clicking outside the canvas (e.g., on the terminal drawer) releases focus.

#### Modified: Sandbox detail page

**File:** `console/src/app/sandboxes/[id]/page.tsx`

When `sandbox.runtime === "desktop"` and the sandbox is live:
- Render `DesktopView` as the main content instead of `TerminalView`.
- Render a collapsible terminal drawer at the bottom.
- The existing "History" and "Use in your app" sidebars remain unchanged.

When `sandbox.runtime !== "desktop"` (any CLI runtime):
- No changes — render `TerminalView` as today.

#### Terminal drawer

Part of the sandbox detail page, rendered below the `DesktopView`.

**States:**
- **Collapsed** (default): A 32px bar showing a chevron-up icon, "Terminal" label, and "click to expand" hint.
- **Expanded**: The bar becomes a drag handle. `TerminalView` renders below it. Height remembered in `sessionStorage`. Max 60% of viewport.

**Interactions:**
- Click the collapsed bar to expand to last-remembered height (default: 200px).
- Drag the handle to resize.
- Click the chevron-down icon or press Escape to collapse.

#### Modified: Boot sandbox panel

**File:** `console/src/components/boot-sandbox-panel.tsx`

When the selected runtime is `desktop`, the `handleBoot` callback merges `VNC_ENABLED: "1"` and `VNC_VIEWONLY: "0"` into the environment variables before calling `createSandbox`. No UI change — this is automatic.

#### VNC toolbar

A thin bar between the page header and the noVNC canvas.

**Contents:**
- Left: connection status dot (amber/green/red) + status text (`connecting...` / `vnc · 1280×800` / `disconnected — retry in 3s`)
- Right: fullscreen toggle button

**Fullscreen:** Uses the browser Fullscreen API on the `DesktopView` container (not the whole page). Escape exits fullscreen.

#### Connection states in the desktop view

| Sandbox state | Desktop view renders |
|---|---|
| `scheduled` / `loading` / `starting` | Boot spinner + cold start timer (same as today) |
| `ready` but VNC connecting | Desktop area with "Connecting to desktop..." centered, amber status dot |
| `ready` and VNC connected | noVNC canvas, green status dot |
| `ready` but VNC 503 | "Desktop environment starting..." with retry, amber status dot |
| `ready` but VNC disconnected | Last frame frozen, red status dot, "Disconnected — retry in Ns" |
| `terminated` | "Sandbox is terminated" (same as today) |

## What doesn't change

- **CLI sandbox types** (node24, python313, agent) — completely unaffected. They render `TerminalView` as today.
- **Plane proxy** — no routing changes. VNC traffic flows through the existing `/{token}/` path.
- **Desktop Dockerfile** — no changes. VNC stack is already installed.
- **Sandbox config panel** — no new VNC-specific options.
- **SDK snippets** — the "Use in your app" sidebar continues to work; desktop SDK methods (`screenshot()`, `click()`, etc.) are already in the quickstart snippets.

## Testing

- **agentd `/v1/vnc`:** Unit test for 503 when websockify not running. Integration test with a mock WebSocket upstream verifying frame relay.
- **ws-relay `/vnc`:** Test that it resolves the sandbox, connects upstream, and relays frames. Test disconnect propagation.
- **DesktopView component:** Test that it renders, shows connection states, handles mount/unmount cleanup.
- **Sandbox detail page:** Test that desktop runtime renders `DesktopView` + drawer, CLI runtime renders `TerminalView`.
- **Boot panel:** Test that desktop runtime auto-injects VNC env vars.

## Dependencies

- `@novnc/novnc` — MIT-licensed noVNC client library. The standard browser VNC client used by OpenStack Horizon, Proxmox, Guacamole, etc. Ships as ES modules.
- No new Rust crates needed — agentd already depends on `tokio-tungstenite` for WebSocket.
