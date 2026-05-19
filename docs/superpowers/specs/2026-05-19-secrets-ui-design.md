# Secrets UI & Credential Binding — Design Spec

**Date:** 2026-05-19
**Status:** Approved
**Scope:** Console UI for proxy-injected secrets, BFF routes, boot-time auto-binding, egress proxy spike

---

## 1. Problem

The platform has two credential mechanisms:

- **Environment variables** — visible to the sandbox as `process.env.*`, configured pre-boot in the spawn config. Already implemented in the console UI.
- **Secrets / credential bindings** — the sandbox never sees the raw value. The egress proxy intercepts outbound requests matching a destination and injects an HTTP header (e.g. `Authorization: Bearer sk-...`). Backend types and controller API exist (`POST /pub/v1/secrets`, `POST /pub/v1/sandboxes/:id/credentials`), but there is no console UI and no egress proxy in the dev fleet.

This spec covers building the secrets UI, wiring the boot flow, and spiking the egress proxy for full-chain verification.

---

## 2. Data Model

### 2.1 New types

```typescript
type SecretEntry = {
  destination: string;    // e.g. "api.openai.com"
  header_name: string;    // e.g. "Authorization"
  header_format: string;  // e.g. "Bearer {value}", defaults to "{value}"
  value: string;          // raw secret, masked in UI after entry
};
```

### 2.2 Changes to SandboxConfigValues

```typescript
type SandboxConfigValues = {
  trust: "trusted" | "untrusted" | "paranoid";
  vcpus: number;
  timeout_ms: number;
  max_idle_ms: number;
  env: Record<string, string>;
  secrets: SecretEntry[];  // NEW
  persistent: boolean;
};
```

`DEFAULT_CONFIG` gains `secrets: []`.

### 2.3 Override counting

`countOverrides` counts secrets as one override when `secrets.length > 0` (same pattern as env). `isDefault` includes `config.secrets.length === 0`.

---

## 3. UI Component — SecretEditor

A new `SecretEditor` subcomponent inside `sandbox-config.tsx`, rendered in a "Secrets" section between "Environment" and "Persistent".

### 3.1 Layout

- **Section label:** "Secrets" with subtitle text "Injected by proxy — sandbox never sees these"
- **Existing entries:** Each row shows:
  - `destination` in cyan monospace (same style as env keys)
  - `=` separator
  - `header_name` in regular text
  - `value` displayed as `********` (masked)
  - X button to remove
- **Draft inputs:** Three fields:
  - `destination` — text input, placeholder "api.example.com"
  - `header` — text input, placeholder "Authorization"
  - `secret` — password input (`type="password"`), placeholder "value"
  - Plus button (disabled when destination or value is empty)
  - Enter key in any field triggers add

### 3.2 Header format auto-detection

When the user enters a value starting with `Bearer ` (case-sensitive):
- `header_format` is set to `"Bearer {value}"`
- The stored `value` is the remainder after `Bearer `

Otherwise:
- `header_format` is `"{value}"`
- The full input is stored as `value`

This keeps the UI to one value field instead of exposing header_format as a separate input.

---

## 4. BFF API Routes

### 4.1 New routes

```
POST /api/secrets
  → proxies to CONTROLLER_URL/pub/v1/secrets
  → body: { value: string, name?: string, tags?: Record<string, string> }
  → response: { id: string, name?: string, created_at: string }

POST /api/sandboxes/:id/credentials
  → proxies to CONTROLLER_URL/pub/v1/sandboxes/:id/credentials
  → body: { secret_id: string, destination: string, header_name: string, header_format?: string }
  → response: { id: string, sandbox_id: string, destination: string, ... }
```

Both routes use the same `authHeaders` pattern as existing sandbox routes.

### 4.2 BFF client methods

```typescript
createSecret(body: { value: string; name?: string }): Promise<{ id: string }>

createCredentialBinding(
  sandboxId: string,
  body: {
    secret_id: string;
    destination: string;
    header_name: string;
    header_format?: string;
  }
): Promise<{ id: string }>
```

### 4.3 React Query hook

```typescript
useBindSecrets(): UseMutationResult<void, Error, {
  sandboxId: string;
  secrets: SecretEntry[];
}>
```

Internally iterates each secret entry: creates the secret, then creates the credential binding.

---

## 5. Boot Flow

In `page.tsx`'s `handleBoot` callback, after `createSandbox` succeeds:

1. If `config.secrets.length === 0`, no change — proceed as before.
2. If secrets are present, call `bindSecrets.mutate({ sandboxId, secrets })`.
3. Binding runs in parallel with terminal connection — don't block terminal access.
4. On binding failure: log to console, show a warning banner in the sandbox toolbar (e.g. "1 secret failed to bind"). Non-fatal — sandbox is still usable.

Secrets are NOT included in the spawn config `env` field. They travel through a separate API path.

---

## 6. SDK Snippet Integration

`SdkSnippet` receives `config` including `secrets`. When secrets are present, the generated code includes binding calls after `Sandbox.create()`.

### 6.1 TypeScript output

```typescript
const sandbox = await Sandbox.create({ ... });

await sandbox.bindSecret({
  destination: "api.openai.com",
  headerName: "Authorization",
  headerFormat: "Bearer {value}",
  value: process.env.OPENAI_API_KEY!,
});
```

### 6.2 Python output

```python
import os
from sandbox_platform import Sandbox, CreateSandboxRequest, ResourcesSpec

sandbox = Sandbox.create(CreateSandboxRequest(...))

sandbox.bind_secret(
    destination="api.openai.com",
    header_name="Authorization",
    header_format="Bearer {value}",
    value=os.environ["OPENAI_API_KEY"],
)
```

The snippet uses `process.env.*` / `os.environ[]` as the value source instead of hardcoding the secret.

The env var name is derived from the destination: `api.openai.com` → `OPENAI_API_KEY` (heuristic: take the second-level domain, uppercase, append `_API_KEY`). If the heuristic produces something awkward, fall back to `SECRET_VALUE`.

---

## 7. Testing

### 7.1 Component tests (Vitest + RTL)

File: `__tests__/components/sandbox-config-secrets.test.tsx`

- SecretEditor renders draft inputs (destination, header, secret)
- Adding a secret entry via Enter key
- Adding via plus button
- Masked value display (shows `********`, not the raw value)
- Removing a secret entry via X
- Empty destination or value prevents add
- Header format auto-detection (`Bearer ` prefix)
- SandboxConfig counts secrets as override
- Reset to defaults clears secrets
- Preserves other config fields when adding secrets

File: `__tests__/components/sdk-snippet-secrets.test.tsx`

- SdkSnippet generates `bindSecret` call when secrets present
- Uses `process.env.*` pattern for TypeScript
- Uses `os.environ[]` pattern for Python
- No bindSecret call when secrets array is empty

### 7.2 BFF API tests (Vitest)

File: `__tests__/api/secrets.test.ts`

- createSecret calls correct controller endpoint
- createCredentialBinding calls correct endpoint with sandbox ID
- Error responses surface as BffError

### 7.3 Live API e2e

- Create a secret via `POST /api/secrets`
- Boot a sandbox via UI
- Bind the secret to the sandbox via `POST /api/sandboxes/:id/credentials`
- Verify binding exists on the controller (check response)
- Verify PushCredential action dispatched (check drone logs)

### 7.4 Full proxy e2e (spike-dependent)

If the egress proxy spike succeeds:
- Echo server in dev fleet receives the injected header
- Sandbox `curl` to echo server shows `Authorization` header it did NOT send

---

## 8. Egress Proxy Spike

Time-boxed effort to wire up proxy-based header injection in the dev fleet.

### 8.1 New dev fleet containers

- **envoy** — Envoy proxy with xDS control plane listener, mounted config
- **echo-server** — Simple HTTP server that echoes received headers back in the response body

### 8.2 Approach

1. Add containers to `dev-fleet/docker-compose.yml`
2. Configure drone containers to route outbound traffic through Envoy (explicit HTTP proxy for the spike — skip iptables/DNAT complexity)
3. Attempt to wire `PushCredential` action → xDS config push → Envoy header injection
4. Test: from sandbox, `curl http://echo-server:8080/` and verify `Authorization` header appears in response

### 8.3 Exit criteria

- **Works:** Full e2e demo from browser UI, document the setup
- **Doesn't work:** Document what's missing, stop at API-level e2e (Section 7.3), track proxy wiring as `Next:` trailer in session-ending commit

---

## 9. Files Changed

| File | Change |
|---|---|
| `console/src/components/sandbox-config.tsx` | Add `SecretEntry` type, `secrets` to config, `SecretEditor` component, update `countOverrides`/`isDefault` |
| `console/src/components/sdk-snippet.tsx` | Generate `bindSecret` calls when secrets present |
| `console/src/app/page.tsx` | Post-boot secret binding in `handleBoot` |
| `console/src/app/api/secrets/route.ts` | New BFF route proxying to controller |
| `console/src/app/api/sandboxes/[id]/credentials/route.ts` | New BFF route for credential binding |
| `console/src/api/client.ts` | Add `createSecret`, `createCredentialBinding` methods |
| `console/src/api/hooks.ts` | Add `useBindSecrets` mutation |
| `console/__tests__/components/sandbox-config-secrets.test.tsx` | SecretEditor component tests |
| `console/__tests__/components/sdk-snippet-secrets.test.tsx` | SDK snippet secret tests |
| `console/__tests__/api/secrets.test.ts` | BFF route tests |
| `dev-fleet/docker-compose.yml` | Add envoy + echo-server containers (spike) |

---

## 10. Out of Scope

- Secret rotation / versioning
- Secret sharing across sandboxes
- Per-destination TLS certificate management
- Revocation UI (can be added later via the existing `DELETE /pub/v1/sandboxes/:id/credentials/:cred_id` endpoint)
- KMS integration for the dev fleet (dev fleet stores secrets in plaintext in Postgres — KMS is a production concern)
