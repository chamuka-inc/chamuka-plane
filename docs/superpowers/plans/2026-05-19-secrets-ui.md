# Secrets UI & Credential Binding — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a secrets UI to the console config panel that auto-creates secrets and credential bindings on sandbox boot, so the egress proxy can inject HTTP headers into outbound requests without the sandbox seeing the raw secret value.

**Architecture:** The console config panel gains a `SecretEditor` component (same pattern as `EnvEditor`). On boot, the BFF creates secrets via `POST /pub/v1/secrets` and binds them via `POST /pub/v1/sandboxes/:id/credentials`. The controller dispatches `PushCredential` to the drone's `EgressPublisher`. The egress proxy spike adds an echo server to the dev fleet to verify header injection end-to-end. A dev-mode migration simplifies the `secret` table to avoid KMS dependency.

**Tech Stack:** Next.js 15, React 19, Vitest, React Testing Library, @testing-library/user-event, Docker Compose

**Design spec:** `docs/superpowers/specs/2026-05-19-secrets-ui-design.md`

---

## Pre-implementation: Contract Verification

The following contracts are already in place and must not be violated:

| Layer | Source of truth | Status |
|---|---|---|
| API types | `crates/common/src/ext/api.rs:156-190` | `CreateSecretRequest`, `CreateSecretResponse`, `CreateCredentialBindingRequest`, `CredentialBinding` — all present |
| JSON schemas | `contracts/schemas/create-secret-*.schema.json`, `create-credential-binding-request.schema.json`, `credential-binding.schema.json` | All present, generated from Rust types |
| SDK types (TS) | `sdks/typescript/src/generated/types.ts:42-69` | Generated. `CreateCredentialBindingRequest`, `CreateSecretRequest`, `CreateSecretResponse`, `CredentialBinding` |
| SDK methods (TS) | `sdks/typescript/src/client.ts:108-176` | `createCredential()`, `deleteCredential()`, `createSecret()`, `deleteSecret()` |
| SDK Sandbox (TS) | `sdks/typescript/src/sandbox.ts:406-418` | `sandbox.createCredential(binding)`, `sandbox.deleteCredential(credentialId)` |
| SDK methods (Py) | `sdks/python/src/sandbox_platform/client.py:97-150` | Same four methods |
| SDK Sandbox (Py) | `sdks/python/src/sandbox_platform/sandbox.py:229-238` | `sandbox.create_credential(binding)`, `sandbox.delete_credential(credential_id)` |
| Controller routes | `crates/plane/src/ext/api_v1.rs:566-760` | All four handlers implemented |
| Wire protocol | `crates/plane-common/src/protocol.rs` | `MessageToDrone::PushCredential(PushCredentialWire)`, `MessageToDrone::RevokeCredential { action_id, binding_id }` |
| Drone handler | `crates/plane/src/drone/extension_handler.rs:89-106` | Dispatches to `EgressPublisher` trait |
| Fixtures | `contracts/fixtures/extension-action/push-credential.json`, `revoke-credential.json` | Present |

**Critical bug found:** `crates/plane/src/database/secret.rs:21` inserts into column `encrypted_value`, but the migration (`20260601000000_add-sandbox-platform.sql:83-96`) defines the `secret` table with KMS columns (`kms_key_id`, `wrapped_data_key`, `nonce`, `ciphertext`, `auth_tag`). `POST /pub/v1/secrets` will fail at runtime. Task 1 fixes this.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `crates/plane/schema/migrations/20260602000000_secret-dev-mode.sql` | Create | Dev-mode migration: add `encrypted_value` column |
| `crates/plane/src/database/secret.rs` | Modify | Fix insert to use `encrypted_value` column |
| `console/src/components/sandbox-config.tsx` | Modify | Add `SecretEntry` type, `secrets` field, `SecretEditor` component |
| `console/src/app/api/secrets/route.ts` | Create | BFF proxy to `POST /pub/v1/secrets` |
| `console/src/app/api/sandboxes/[id]/credentials/route.ts` | Create | BFF proxy to `POST /pub/v1/sandboxes/:id/credentials` |
| `console/src/api/client.ts` | Modify | Add `createSecret()`, `createCredentialBinding()` methods |
| `console/src/api/hooks.ts` | Modify | Add `useBindSecrets()` mutation |
| `console/src/app/page.tsx` | Modify | Call `useBindSecrets` in boot flow |
| `console/src/components/sdk-snippet.tsx` | Modify | Generate credential binding code when secrets present |
| `console/__tests__/components/sandbox-config-secrets.test.tsx` | Create | SecretEditor component tests |
| `console/__tests__/components/sdk-snippet-secrets.test.tsx` | Create | SDK snippet secret rendering tests |
| `console/__tests__/api/secrets.test.ts` | Create | BFF route tests |
| `dev-fleet/docker-compose.yml` | Modify | Add echo-server container |

---

## Task 1: Fix secret table column mismatch for dev mode

The `secret` table uses KMS envelope encryption columns (production design), but the DB layer code references a simpler `encrypted_value` column. For the dev fleet (no KMS), add a migration that adds this column so the existing code works.

**Files:**
- Create: `crates/plane/schema/migrations/20260602000000_secret-dev-mode.sql`
- Reference: `crates/plane/src/database/secret.rs:19-24`
- Reference: `crates/plane/schema/migrations/20260601000000_add-sandbox-platform.sql:83-96`

- [ ] **Step 1: Write the dev-mode migration**

```sql
-- Dev-mode secret storage: adds a plaintext column for use without KMS.
-- Production deployments use the KMS envelope columns (kms_key_id, wrapped_data_key, etc.).
-- This column is nullable so existing rows (if any) are unaffected.
ALTER TABLE secret ADD COLUMN IF NOT EXISTS encrypted_value TEXT;

-- Make the KMS columns nullable for dev mode (they were NOT NULL).
ALTER TABLE secret ALTER COLUMN kms_key_id DROP NOT NULL;
ALTER TABLE secret ALTER COLUMN wrapped_data_key DROP NOT NULL;
ALTER TABLE secret ALTER COLUMN nonce DROP NOT NULL;
ALTER TABLE secret ALTER COLUMN ciphertext DROP NOT NULL;
ALTER TABLE secret ALTER COLUMN auth_tag DROP NOT NULL;
```

- [ ] **Step 2: Verify the migration applies cleanly**

Run:
```bash
cd dev-fleet && docker compose exec postgres psql -U postgres -c "\d secret"
```

Then restart the controller so it picks up the migration:
```bash
docker compose restart controller
sleep 3
docker compose logs controller --tail 20 | grep -i "migrat"
```

Expected: migration runs, `encrypted_value` column appears in the table.

- [ ] **Step 3: Verify secret creation works end-to-end**

```bash
curl -s -X POST http://localhost:8080/pub/v1/secrets \
  -H 'Content-Type: application/json' \
  -H 'x-auth-tenant: tenant-default' \
  -H 'x-auth-user: admin' \
  -d '{"value": "sk-test-123", "name": "test-key"}' | python3 -m json.tool
```

Expected: `201 Created` with `{ "id": "secret-...", "name": "test-key", "created_at": "..." }`.

- [ ] **Step 4: Commit**

```bash
git add crates/plane/schema/migrations/20260602000000_secret-dev-mode.sql
git commit -m "plane: fix: add dev-mode migration for secret table

The secret.rs DB layer inserts into encrypted_value, but the
original migration only defined KMS envelope columns. Add a
dev-mode migration that adds encrypted_value and relaxes NOT
NULL on KMS columns so secrets work without a KMS provider."
```

---

## Task 2: SecretEditor component — data model and rendering

Add the `SecretEntry` type, update `SandboxConfigValues`, and build the `SecretEditor` component with masked values.

**Files:**
- Modify: `console/src/components/sandbox-config.tsx`
- Create: `console/__tests__/components/sandbox-config-secrets.test.tsx`

- [ ] **Step 1: Write the failing tests**

Create `console/__tests__/components/sandbox-config-secrets.test.tsx`:

```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import {
  SandboxConfig,
  DEFAULT_CONFIG,
  type SandboxConfigValues,
} from "@/components/sandbox-config";

function renderConfig(
  overrides: Partial<SandboxConfigValues> = {},
  onChange?: (v: SandboxConfigValues) => void,
) {
  const value = { ...DEFAULT_CONFIG, ...overrides };
  const handleChange = onChange ?? vi.fn();
  return { ...render(<SandboxConfig value={value} onChange={handleChange} />), handleChange };
}

async function openPanel(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByText("Configure"));
}

describe("SandboxConfig — Secrets", () => {
  let user: ReturnType<typeof userEvent.setup>;
  beforeEach(() => { user = userEvent.setup(); });

  it("shows secrets section when config panel is opened", async () => {
    renderConfig();
    await openPanel(user);
    expect(screen.getByText("Secrets")).toBeDefined();
    expect(screen.getByText(/injected by proxy/i)).toBeDefined();
  });

  it("adds a secret entry via Enter key", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("api.example.com"), "api.openai.com");
    await user.type(screen.getByPlaceholderText("Authorization"), "Authorization");
    await user.type(screen.getByPlaceholderText("secret value"), "Bearer sk-test-123");
    await user.keyboard("{Enter}");

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        secrets: [
          {
            destination: "api.openai.com",
            header_name: "Authorization",
            header_format: "Bearer {value}",
            value: "sk-test-123",
          },
        ],
      }),
    );
  });

  it("masks the secret value after entry", async () => {
    renderConfig({
      secrets: [
        { destination: "api.openai.com", header_name: "Authorization", header_format: "{value}", value: "sk-secret" },
      ],
    });
    await openPanel(user);

    expect(screen.getByText("api.openai.com")).toBeDefined();
    expect(screen.queryByText("sk-secret")).toBeNull();
    expect(screen.getByText(/\*{6,}/)).toBeDefined();
  });

  it("removes a secret entry when clicking X", async () => {
    const handleChange = vi.fn();
    renderConfig({
      secrets: [
        { destination: "api.openai.com", header_name: "Authorization", header_format: "{value}", value: "sk-1" },
        { destination: "api.stripe.com", header_name: "Authorization", header_format: "{value}", value: "sk-2" },
      ],
    }, handleChange);
    await openPanel(user);

    const openaiRow = screen.getByText("api.openai.com").closest("div")!;
    await user.click(within(openaiRow).getByRole("button"));

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({
        secrets: [
          { destination: "api.stripe.com", header_name: "Authorization", header_format: "{value}", value: "sk-2" },
        ],
      }),
    );
  });

  it("prevents add when destination is empty", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("secret value"), "some-secret");
    await user.keyboard("{Enter}");

    expect(handleChange).not.toHaveBeenCalled();
  });

  it("prevents add when value is empty", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("api.example.com"), "api.example.com");
    await user.keyboard("{Enter}");

    expect(handleChange).not.toHaveBeenCalled();
  });

  it("auto-detects Bearer prefix in header format", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("api.example.com"), "api.openai.com");
    await user.type(screen.getByPlaceholderText("Authorization"), "Authorization");
    await user.type(screen.getByPlaceholderText("secret value"), "Bearer sk-my-key");
    await user.keyboard("{Enter}");

    const call = handleChange.mock.calls[0][0];
    expect(call.secrets[0].header_format).toBe("Bearer {value}");
    expect(call.secrets[0].value).toBe("sk-my-key");
  });

  it("uses plain {value} format when no Bearer prefix", async () => {
    const handleChange = vi.fn();
    renderConfig({}, handleChange);
    await openPanel(user);

    await user.type(screen.getByPlaceholderText("api.example.com"), "api.stripe.com");
    await user.type(screen.getByPlaceholderText("Authorization"), "X-Api-Key");
    await user.type(screen.getByPlaceholderText("secret value"), "raw-key-123");
    await user.keyboard("{Enter}");

    const call = handleChange.mock.calls[0][0];
    expect(call.secrets[0].header_format).toBe("{value}");
    expect(call.secrets[0].value).toBe("raw-key-123");
  });

  it("counts secrets as an override", () => {
    renderConfig({
      secrets: [
        { destination: "api.openai.com", header_name: "Authorization", header_format: "{value}", value: "sk-1" },
      ],
    });
    const badge = screen.getByText("1");
    expect(badge).toBeDefined();
  });

  it("clears secrets on reset to defaults", async () => {
    const handleChange = vi.fn();
    renderConfig({
      secrets: [
        { destination: "api.openai.com", header_name: "Authorization", header_format: "{value}", value: "sk-1" },
      ],
    }, handleChange);
    await openPanel(user);

    await user.click(screen.getByText("Reset to defaults"));

    expect(handleChange).toHaveBeenCalledWith(
      expect.objectContaining({ secrets: [] }),
    );
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd console && npx vitest run __tests__/components/sandbox-config-secrets.test.tsx`

Expected: Multiple failures — `SecretEntry` type doesn't exist, `secrets` not in `DEFAULT_CONFIG`, no "Secrets" section in UI.

- [ ] **Step 3: Add SecretEntry type and update SandboxConfigValues**

In `console/src/components/sandbox-config.tsx`, add the type after line 5 and update `SandboxConfigValues` and `DEFAULT_CONFIG`:

```tsx
export type SecretEntry = {
  destination: string;
  header_name: string;
  header_format: string;
  value: string;
};

export type SandboxConfigValues = {
  trust: "trusted" | "untrusted" | "paranoid";
  vcpus: number;
  timeout_ms: number;
  max_idle_ms: number;
  env: Record<string, string>;
  secrets: SecretEntry[];
  persistent: boolean;
};

export const DEFAULT_CONFIG: SandboxConfigValues = {
  trust: "untrusted",
  vcpus: 2,
  timeout_ms: 300_000,
  max_idle_ms: 60_000,
  env: {},
  secrets: [],
  persistent: false,
};
```

- [ ] **Step 4: Build the SecretEditor component**

Add after the `EnvEditor` function (after line 115):

```tsx
function SecretEditor({
  secrets,
  onChange,
}: {
  secrets: SecretEntry[];
  onChange: (secrets: SecretEntry[]) => void;
}) {
  const [draftDest, setDraftDest] = useState("");
  const [draftHeader, setDraftHeader] = useState("");
  const [draftValue, setDraftValue] = useState("");

  const handleAdd = useCallback(() => {
    const dest = draftDest.trim();
    const val = draftValue;
    if (!dest || !val) return;

    let headerFormat = "{value}";
    let storedValue = val;
    if (val.startsWith("Bearer ")) {
      headerFormat = "Bearer {value}";
      storedValue = val.slice(7);
    }

    onChange([
      ...secrets,
      {
        destination: dest,
        header_name: draftHeader.trim() || "Authorization",
        header_format: headerFormat,
        value: storedValue,
      },
    ]);
    setDraftDest("");
    setDraftHeader("");
    setDraftValue("");
  }, [draftDest, draftHeader, draftValue, secrets, onChange]);

  const handleRemove = useCallback(
    (index: number) => {
      onChange(secrets.filter((_, i) => i !== index));
    },
    [secrets, onChange],
  );

  return (
    <div className="space-y-1.5">
      {secrets.map((s, i) => (
        <div key={`${s.destination}-${s.header_name}-${i}`} className="flex items-center gap-1.5">
          <span className="text-xs font-mono text-accent-cyan truncate min-w-0">{s.destination}</span>
          <span className="text-xs text-text-4">/</span>
          <span className="text-xs text-text-2 truncate">{s.header_name}</span>
          <span className="text-xs text-text-4">=</span>
          <span className="text-xs font-mono text-text-3 truncate flex-1 min-w-0">{"********"}</span>
          <button
            onClick={() => handleRemove(i)}
            className="text-text-4 hover:text-status-red transition-colors shrink-0 p-0.5"
          >
            <X size={10} />
          </button>
        </div>
      ))}
      <div className="flex items-center gap-1.5">
        <input
          type="text"
          value={draftDest}
          onChange={(e) => setDraftDest(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
          placeholder="api.example.com"
          className="w-28 text-xs font-mono bg-bg-1 border border-border rounded px-1.5 py-1 text-text placeholder:text-text-4 focus:outline-none focus:border-border-light"
        />
        <input
          type="text"
          value={draftHeader}
          onChange={(e) => setDraftHeader(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
          placeholder="Authorization"
          className="w-20 text-xs font-mono bg-bg-1 border border-border rounded px-1.5 py-1 text-text placeholder:text-text-4 focus:outline-none focus:border-border-light"
        />
        <input
          type="password"
          value={draftValue}
          onChange={(e) => setDraftValue(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); }}
          placeholder="secret value"
          className="flex-1 min-w-0 text-xs font-mono bg-bg-1 border border-border rounded px-1.5 py-1 text-text placeholder:text-text-4 focus:outline-none focus:border-border-light"
        />
        <button
          onClick={handleAdd}
          disabled={!draftDest.trim() || !draftValue}
          className="text-text-3 hover:text-text disabled:text-text-4 transition-colors shrink-0 p-0.5"
        >
          <Plus size={12} />
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Render SecretEditor in the config panel**

In the `SandboxConfig` component, add a "Secrets" section between the "Env vars" section (after line 240) and "Persistent toggle":

```tsx
          {/* Secrets */}
          <div className="p-3 space-y-2">
            <div>
              <label className="text-xs text-text-3 font-medium">Secrets</label>
              <span className="text-[10px] text-text-4 ml-2">Injected by proxy — sandbox never sees these</span>
            </div>
            <SecretEditor secrets={value.secrets} onChange={(secrets) => update({ secrets })} />
          </div>
```

- [ ] **Step 6: Update `isDefault` and `countOverrides`**

In `isDefault` (line 117), add: `config.secrets.length === 0 &&`

In `countOverrides` (line 128), add: `if (config.secrets.length > 0) n++;`

- [ ] **Step 7: Run tests**

Run: `cd console && npx vitest run __tests__/components/sandbox-config-secrets.test.tsx`

Expected: All 9 tests pass.

- [ ] **Step 8: Run existing tests for regression**

Run: `cd console && npx vitest run`

Expected: All existing tests still pass (sandbox-row, client, hooks, format, sandbox-config-env).

- [ ] **Step 9: Commit**

```bash
git add console/src/components/sandbox-config.tsx console/__tests__/components/sandbox-config-secrets.test.tsx
git commit -m "console: feat: add SecretEditor component to config panel

Adds SecretEntry type and SecretEditor to the sandbox config panel,
positioned between Environment and Persistent. Values are masked
after entry. Bearer prefix is auto-detected for header_format.
Secrets count as a config override in the badge."
```

---

## Task 3: BFF API routes for secrets and credential bindings

Create the two server-side API routes the console uses to proxy to the controller.

**Files:**
- Create: `console/src/app/api/secrets/route.ts`
- Create: `console/src/app/api/sandboxes/[id]/credentials/route.ts`
- Create: `console/__tests__/api/secrets.test.ts`

- [ ] **Step 1: Write the failing BFF tests**

Create `console/__tests__/api/secrets.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { createBffClient } from "@/api/client";

describe("BFF secret routes", () => {
  beforeEach(() => { vi.restoreAllMocks(); });

  it("createSecret calls /api/secrets", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ id: "secret-abc", created_at: "2026-05-19T00:00:00Z" }),
    });

    const client = createBffClient();
    const result = await client.createSecret({ value: "sk-test", name: "my-key" });

    expect(result.id).toBe("secret-abc");
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/secrets",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ value: "sk-test", name: "my-key" }),
      }),
    );
  });

  it("createCredentialBinding calls /api/sandboxes/:id/credentials", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ id: "cred-xyz" }),
    });

    const client = createBffClient();
    const result = await client.createCredentialBinding("sb-123", {
      secret_id: "secret-abc",
      destination: "api.openai.com",
      header_name: "Authorization",
      header_format: "Bearer {value}",
    });

    expect(result.id).toBe("cred-xyz");
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/sandboxes/sb-123/credentials",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          secret_id: "secret-abc",
          destination: "api.openai.com",
          header_name: "Authorization",
          header_format: "Bearer {value}",
        }),
      }),
    );
  });

  it("createSecret surfaces errors as BffError", async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      statusText: "Internal Server Error",
      json: () => Promise.resolve({ error: { code: "INTERNAL", message: "KMS unavailable" } }),
    });

    const client = createBffClient();
    await expect(client.createSecret({ value: "sk-test" }))
      .rejects.toThrow("INTERNAL");
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd console && npx vitest run __tests__/api/secrets.test.ts`

Expected: FAIL — `createSecret` and `createCredentialBinding` don't exist on the client.

- [ ] **Step 3: Add client methods**

In `console/src/api/client.ts`, add these methods inside the object returned by `createBffClient()` (after `terminateSandbox`, before the closing `};`):

```typescript
    createSecret(body: { value: string; name?: string }): Promise<{ id: string; created_at: string }> {
      return request("/api/secrets", {
        method: "POST",
        body: JSON.stringify(body),
      });
    },

    createCredentialBinding(
      sandboxId: string,
      body: {
        secret_id: string;
        destination: string;
        header_name: string;
        header_format?: string;
      },
    ): Promise<{ id: string }> {
      return request(`/api/sandboxes/${encodeURIComponent(sandboxId)}/credentials`, {
        method: "POST",
        body: JSON.stringify(body),
      });
    },
```

- [ ] **Step 4: Run tests**

Run: `cd console && npx vitest run __tests__/api/secrets.test.ts`

Expected: All 3 tests pass.

- [ ] **Step 5: Create the BFF secrets route**

Create `console/src/app/api/secrets/route.ts`:

```typescript
import { NextRequest, NextResponse } from "next/server";
import { CONTROLLER_URL, authHeaders, bffError } from "@/lib/controller";

export async function POST(request: NextRequest) {
  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return bffError("BAD_REQUEST", "Invalid JSON", 400);
  }

  const res = await fetch(`${CONTROLLER_URL}/pub/v1/secrets`, {
    method: "POST",
    headers: { ...authHeaders, "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  let data;
  try {
    data = await res.json();
  } catch {
    return bffError("UPSTREAM_ERROR", `Controller returned ${res.status}`, 502);
  }

  return NextResponse.json(data, { status: res.status });
}
```

- [ ] **Step 6: Create the BFF credential binding route**

Create `console/src/app/api/sandboxes/[id]/credentials/route.ts`:

```typescript
import { NextRequest, NextResponse } from "next/server";
import { CONTROLLER_URL, authHeaders, bffError } from "@/lib/controller";

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;

  let body: unknown;
  try {
    body = await request.json();
  } catch {
    return bffError("BAD_REQUEST", "Invalid JSON", 400);
  }

  const res = await fetch(
    `${CONTROLLER_URL}/pub/v1/sandboxes/${encodeURIComponent(id)}/credentials`,
    {
      method: "POST",
      headers: { ...authHeaders, "Content-Type": "application/json" },
      body: JSON.stringify(body),
    },
  );

  let data;
  try {
    data = await res.json();
  } catch {
    return bffError("UPSTREAM_ERROR", `Controller returned ${res.status}`, 502);
  }

  return NextResponse.json(data, { status: res.status });
}
```

- [ ] **Step 7: Run all tests**

Run: `cd console && npx vitest run`

Expected: All tests pass including the new secrets tests.

- [ ] **Step 8: Commit**

```bash
git add console/src/app/api/secrets/route.ts \
       console/src/app/api/sandboxes/\[id\]/credentials/route.ts \
       console/src/api/client.ts \
       console/__tests__/api/secrets.test.ts
git commit -m "console: feat: add BFF routes for secrets and credential bindings

POST /api/secrets proxies to controller's POST /pub/v1/secrets.
POST /api/sandboxes/:id/credentials proxies to the controller's
credential binding endpoint. Both use the same auth header pattern
as existing sandbox routes."
```

---

## Task 4: Boot flow — auto-bind secrets after sandbox creation

Wire the boot flow to create secrets and bind them to the sandbox automatically.

**Files:**
- Modify: `console/src/api/hooks.ts`
- Modify: `console/src/app/page.tsx`

- [ ] **Step 1: Add useBindSecrets hook**

In `console/src/api/hooks.ts`, add after `useTerminateSandbox`:

```typescript
import type { SecretEntry } from "@/components/sandbox-config";

export function useBindSecrets() {
  return useMutation<
    void,
    Error,
    { sandboxId: string; secrets: SecretEntry[] }
  >({
    mutationFn: async ({ sandboxId, secrets }) => {
      for (const secret of secrets) {
        const { id: secretId } = await client.createSecret({
          value: secret.header_format === "{value}"
            ? secret.value
            : secret.header_format.replace("{value}", secret.value),
          name: secret.destination,
        });

        await client.createCredentialBinding(sandboxId, {
          secret_id: secretId,
          destination: secret.destination,
          header_name: secret.header_name,
          header_format: secret.header_format,
        });
      }
    },
  });
}
```

- [ ] **Step 2: Wire boot flow in page.tsx**

In `console/src/app/page.tsx`, add the import and hook:

```typescript
import { useCreateSandbox, useTerminateSandbox, useBindSecrets } from "@/api/hooks";
```

Inside `Playground()`, add after `const terminateSandbox = useTerminateSandbox();`:

```typescript
const bindSecrets = useBindSecrets();
```

In `handleBoot`'s `onSuccess` callback (line 59), after `setActive(...)` and before `setBooting(false)`, add:

```typescript
            if (config.secrets.length > 0) {
              bindSecrets.mutate(
                { sandboxId: data.sandbox.id, secrets: config.secrets },
                {
                  onError: (err) => {
                    console.error("Failed to bind secrets:", err);
                  },
                },
              );
            }
```

- [ ] **Step 3: Run all tests**

Run: `cd console && npx vitest run`

Expected: All tests pass. The boot flow change is not covered by unit tests (it requires a running controller). We verify it manually in Task 7.

- [ ] **Step 4: Commit**

```bash
git add console/src/api/hooks.ts console/src/app/page.tsx
git commit -m "console: feat: auto-bind secrets on sandbox boot

After createSandbox succeeds, useBindSecrets iterates each
configured secret entry: creates the secret via POST /api/secrets,
then binds it via POST /api/sandboxes/:id/credentials. Binding
is non-blocking — terminal access is not delayed. Failures log
to console."
```

---

## Task 5: SDK snippet integration

Generate `createSecret` + `createCredential` calls in the SDK snippet when secrets are configured. Must use the actual SDK API — `SandboxClient.createSecret()` + `Sandbox.createCredential()` (TS) and `SandboxClient.create_secret()` + `Sandbox.create_credential()` (Py).

**Files:**
- Modify: `console/src/components/sdk-snippet.tsx`
- Create: `console/__tests__/components/sdk-snippet-secrets.test.tsx`

- [ ] **Step 1: Write the failing tests**

Create `console/__tests__/components/sdk-snippet-secrets.test.tsx`:

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { SdkSnippet } from "@/components/sdk-snippet";
import { DEFAULT_CONFIG, type SandboxConfigValues } from "@/components/sandbox-config";

const baseProps = {
  runtime: "node24",
  image: "sandbox-runtime:node24-latest",
  sandboxId: "sb-test",
};

function configWith(overrides: Partial<SandboxConfigValues>): SandboxConfigValues {
  return { ...DEFAULT_CONFIG, ...overrides };
}

describe("SdkSnippet — secrets", () => {
  it("generates no credential code when secrets is empty", () => {
    render(<SdkSnippet {...baseProps} config={configWith({ secrets: [] })} />);
    const code = screen.getByRole("code")?.textContent ?? "";
    expect(code).not.toContain("createSecret");
    expect(code).not.toContain("createCredential");
  });

  it("generates TypeScript credential binding code", () => {
    render(
      <SdkSnippet
        {...baseProps}
        config={configWith({
          secrets: [
            {
              destination: "api.openai.com",
              header_name: "Authorization",
              header_format: "Bearer {value}",
              value: "sk-test",
            },
          ],
        })}
      />,
    );
    const code = screen.getByRole("code")?.textContent ?? "";
    expect(code).toContain("createSecret");
    expect(code).toContain("createCredential");
    expect(code).toContain("api.openai.com");
    expect(code).toContain("process.env.");
    expect(code).not.toContain("sk-test");
  });

  it("generates Python credential binding code", async () => {
    const { unmount } = render(
      <SdkSnippet
        {...baseProps}
        config={configWith({
          secrets: [
            {
              destination: "api.openai.com",
              header_name: "Authorization",
              header_format: "Bearer {value}",
              value: "sk-test",
            },
          ],
        })}
      />,
    );
    // Switch to Python
    const pyButton = screen.getByText("Python");
    const userEvent = await import("@testing-library/user-event");
    const user = userEvent.default.setup();
    await user.click(pyButton);

    const code = screen.getByRole("code")?.textContent ?? "";
    expect(code).toContain("create_secret");
    expect(code).toContain("create_credential");
    expect(code).toContain("api.openai.com");
    expect(code).toContain("os.environ");
    expect(code).not.toContain("sk-test");

    unmount();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd console && npx vitest run __tests__/components/sdk-snippet-secrets.test.tsx`

Expected: FAIL — no credential code generated yet. Also need to add `role="code"` to the `<code>` element.

- [ ] **Step 3: Add role="code" to the code element**

In `console/src/components/sdk-snippet.tsx`, change the `<code>` tag (line 194) to:

```tsx
        <code role="code">{code}</code>
```

- [ ] **Step 4: Update generateTs to include credential binding code**

In `console/src/components/sdk-snippet.tsx`, modify `generateTs` to add credential lines after the `Sandbox.create` block. Add before the `const cmds` line (around line 70):

```typescript
  if (props.config?.secrets?.length) {
    lines.push(``);
    for (const s of props.config.secrets) {
      const envVar = deriveEnvVarName(s.destination);
      lines.push(`const { id: ${varName(s.destination)}SecretId } = await client.createSecret({`);
      lines.push(`  value: process.env.${envVar}!,`);
      if (s.destination) lines.push(`  name: ${JSON.stringify(s.destination)},`);
      lines.push(`});`);
      lines.push(`await sandbox.createCredential({`);
      lines.push(`  secret_id: ${varName(s.destination)}SecretId,`);
      lines.push(`  destination: ${JSON.stringify(s.destination)},`);
      lines.push(`  header_name: ${JSON.stringify(s.header_name)},`);
      if (s.header_format !== "{value}") {
        lines.push(`  header_format: ${JSON.stringify(s.header_format)},`);
      }
      lines.push(`});`);
    }
  }
```

Add these helpers at the top of the file (after the imports):

```typescript
function deriveEnvVarName(destination: string): string {
  const parts = destination.replace(/^api\./, "").split(".");
  const base = parts[0].toUpperCase().replace(/[^A-Z0-9]/g, "_");
  return `${base}_API_KEY`;
}

function varName(destination: string): string {
  const parts = destination.replace(/^api\./, "").split(".");
  return parts[0].replace(/[^a-zA-Z0-9]/g, "").toLowerCase();
}
```

Also update the import at line 1 to add `SandboxClient`:

```typescript
const lines = [
    `import { Sandbox, SandboxClient } from "@sandbox-platform/sdk";`,
```

But only when secrets are present. Modify the import line logic:

```typescript
  const hasSecrets = (props.config?.secrets?.length ?? 0) > 0;
  const lines = [
    hasSecrets
      ? `import { Sandbox, SandboxClient } from "@sandbox-platform/sdk";`
      : `import { Sandbox } from "@sandbox-platform/sdk";`,
    ``,
  ];

  if (hasSecrets) {
    lines.push(`const client = new SandboxClient({ token: process.env.SANDBOX_TOKEN! });`);
    lines.push(``);
  }
```

- [ ] **Step 5: Update generatePy to include credential binding code**

In `generatePy`, after the `Sandbox.create(...)` block and before `appendPyExecs`:

```typescript
  if (props.config?.secrets?.length) {
    lines.push(``);
    for (const s of props.config.secrets) {
      const envVar = deriveEnvVarName(s.destination);
      lines.push(`secret = client.create_secret(CreateSecretRequest(`);
      lines.push(`    value=os.environ[${JSON.stringify(envVar)}],`);
      if (s.destination) lines.push(`    name=${JSON.stringify(s.destination)},`);
      lines.push(`))`);
      lines.push(`sandbox.create_credential(CreateCredentialBindingRequest(`);
      lines.push(`    secret_id=secret.id,`);
      lines.push(`    destination=${JSON.stringify(s.destination)},`);
      lines.push(`    header_name=${JSON.stringify(s.header_name)},`);
      if (s.header_format !== "{value}") {
        lines.push(`    header_format=${JSON.stringify(s.header_format)},`);
      }
      lines.push(`))`);
    }
  }
```

Update the Python imports to include `os` and the credential types when secrets are present:

```typescript
  const hasSecrets = (props.config?.secrets?.length ?? 0) > 0;
  const imports = [
    `from sandbox_platform import Sandbox, SandboxClient, CreateSandboxRequest, ResourcesSpec`,
  ];
  if (hasSecrets) {
    imports.push(`from sandbox_platform import CreateSecretRequest, CreateCredentialBindingRequest`);
    imports.push(`import os`);
  }

  const lines = [...imports, ``];

  if (hasSecrets) {
    lines.push(`client = SandboxClient(token=os.environ["SANDBOX_TOKEN"])`);
    lines.push(``);
  }
```

- [ ] **Step 6: Run tests**

Run: `cd console && npx vitest run __tests__/components/sdk-snippet-secrets.test.tsx`

Expected: All 3 tests pass.

- [ ] **Step 7: Run all tests for regression**

Run: `cd console && npx vitest run`

Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add console/src/components/sdk-snippet.tsx console/__tests__/components/sdk-snippet-secrets.test.tsx
git commit -m "console: feat: generate credential binding code in SDK snippets

When secrets are configured, the TypeScript and Python snippets
include client.createSecret() + sandbox.createCredential() calls
using the actual SDK API. Secret values reference process.env /
os.environ rather than hardcoding. Env var names derived from
the destination domain."
```

---

## Task 6: Echo server for egress proxy spike

Add an echo server to the dev fleet that logs received headers, so we can verify header injection.

**Files:**
- Modify: `dev-fleet/docker-compose.yml`

- [ ] **Step 1: Add echo-server to docker-compose**

Add to `dev-fleet/docker-compose.yml` before the `volumes:` section:

```yaml
  echo-server:
    image: hashicorp/http-echo
    container_name: fleet-echo-server
    restart: unless-stopped
    command: ["-listen=:8080", "-text=echo-ok"]
    ports:
      - "127.0.0.1:8888:8080"
```

Wait — `hashicorp/http-echo` doesn't echo headers. Use a one-liner Node container instead. Replace with:

```yaml
  echo-server:
    image: node:24-slim
    container_name: fleet-echo-server
    restart: unless-stopped
    command:
      - node
      - -e
      - |
        require('http').createServer((req, res) => {
          const body = JSON.stringify({ method: req.method, url: req.url, headers: req.headers }, null, 2);
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(body);
        }).listen(8080, () => console.log('echo-server on :8080'));
    ports:
      - "127.0.0.1:8888:8080"
    extra_hosts:
      - "host.docker.internal:host-gateway"
```

- [ ] **Step 2: Start the echo server**

```bash
cd dev-fleet && docker compose up -d echo-server
```

- [ ] **Step 3: Verify the echo server works**

```bash
curl -s http://localhost:8888/ -H "Authorization: Bearer test-token" | python3 -m json.tool
```

Expected: JSON response containing `"authorization": "bearer test-token"` in the `headers` object.

- [ ] **Step 4: Verify sandbox can reach the echo server**

Spawn a sandbox and curl the echo server from inside:

```bash
# Use an existing sandbox token or spawn a new one
TOKEN=$(curl -s -X POST http://localhost:8080/ctrl/connect \
  -H 'Content-Type: application/json' \
  -d '{
    "key": {"name": "echo-test-'$(date +%s)'", "namespace": "dev", "tag": ""},
    "spawn_config": {
      "cluster": "localhost:9090",
      "pool": "trusted",
      "executable": {"image": "sandbox-runtime:node24-latest", "env": {}},
      "lifetime_limit_seconds": 120
    }
  }' | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])")

sleep 2

curl -s "http://localhost:9090/$TOKEN/v1/exec" \
  -H "Host: localhost:9090" \
  -H "Content-Type: application/json" \
  -d '{"cmd":"/bin/sh","args":["-c","curl -s http://host.docker.internal:8888/"],"env":{},"timeout_ms":5000}' \
  | python3 -m json.tool
```

Expected: stdout contains the echo response with no authorization header (sandbox didn't send one).

- [ ] **Step 5: Commit**

```bash
git add dev-fleet/docker-compose.yml
git commit -m "dev-fleet: feat: add echo-server for header injection testing

Node.js HTTP server that echoes request method, URL, and all
headers as JSON. Used to verify egress proxy header injection
without relying on external services."
```

---

## Task 7: Live e2e — secrets API flow through the console UI

Verify the full flow: set secrets in the console UI, boot a sandbox, confirm bindings are created via the controller API.

**Files:** None (manual verification)

- [ ] **Step 1: Start the console dev server (if not running)**

```bash
cd console && npm run dev:next &
```

- [ ] **Step 2: Open the console in the browser**

Navigate to `http://localhost:3100`. Click "Configure". Verify the "Secrets" section appears between "Environment" and "Persistent" with the subtitle "Injected by proxy — sandbox never sees these".

- [ ] **Step 3: Add a secret**

Enter:
- Destination: `api.openai.com`
- Header: `Authorization`
- Secret value: `Bearer sk-test-secret-value`

Press Enter. Verify: the entry appears with `api.openai.com / Authorization = ********`.

- [ ] **Step 4: Boot a Node.js 24 sandbox**

Click "Node.js 24". Wait for the sandbox to boot and terminal to appear.

- [ ] **Step 5: Verify secret was created via controller API**

```bash
# Check the controller logs for secret creation
docker compose -f dev-fleet/docker-compose.yml logs controller --tail 50 | grep -i "secret"
```

Expected: Log entries showing secret creation and credential binding.

- [ ] **Step 6: Verify the sandbox doesn't have the secret as an env var**

In the terminal, run:
```
echo $OPENAI_API_KEY
```

Expected: Empty output — secrets are NOT env vars.

- [ ] **Step 7: Click "Use in your app" and verify the SDK snippet**

Click the "Use in your app" button. Verify the TypeScript snippet includes:
- `import { Sandbox, SandboxClient }`
- `client.createSecret({ value: process.env.OPENAI_API_KEY! })`
- `sandbox.createCredential({ secret_id: ..., destination: "api.openai.com", ... })`

Switch to Python and verify equivalent code with `os.environ`.

- [ ] **Step 8: Document results**

Note whether the full flow succeeded or any issues were encountered. If the egress proxy spike (Task 6) showed the echo server is reachable from sandboxes, attempt to verify header injection appears in echo-server response.

---

## Task 8: Egress proxy spike — attempt header injection

Time-boxed spike: attempt to wire the `PushCredential` action through to actual header injection on outbound requests. If it doesn't wire up cleanly within the spike, document what's missing and stop.

**Files:**
- Reference: `crates/plane/src/drone/extension_handler.rs:8-23` (EgressPublisher trait)
- Reference: `crates/egress-proxy/` (if it exists)

- [ ] **Step 1: Check if egress-proxy crate exists**

```bash
ls -la crates/egress-proxy/src/ 2>/dev/null || echo "NOT FOUND"
```

If it exists, read the implementation. If not, this spike is about standing up the first version.

- [ ] **Step 2: Assess what's needed**

The drone's `EgressPublisher` trait has two methods: `push_credential()` and `revoke_credential()`. The current implementation in the dev fleet likely uses a no-op publisher. To get header injection working we need:

1. A real `EgressPublisher` implementation that stores credential bindings in memory
2. An HTTP proxy that sandboxes route outbound traffic through
3. The proxy checks destinations against stored bindings and injects headers

For the spike, the simplest approach is an explicit HTTP proxy (not transparent MITM) — the sandbox uses `HTTP_PROXY` env var to route through it.

- [ ] **Step 3: Evaluate and decide**

If wiring the proxy is straightforward (< 1 hour), proceed. If it requires significant Envoy config, xDS protocol implementation, or iptables changes, stop here and document what's needed.

**Exit criteria:**
- **Success:** From sandbox, `curl http://echo-server:8080/` through the proxy shows an injected `Authorization` header the sandbox never sent
- **Stop:** Document the gap in the commit message's `Next:` trailer

- [ ] **Step 4: Commit results**

Whether the spike succeeded or not, commit with appropriate trailers:

If successful:
```bash
git commit -m "egress-proxy: spike: header injection via explicit proxy

<description of what was wired up>

Phase: secrets-ui
Next: Polish egress proxy, add TLS MITM for transparent injection"
```

If stopped:
```bash
git commit -m "docs: spike: egress proxy assessment

<description of what's missing>

Phase: secrets-ui
Next: Implement EgressPublisher with in-memory credential store,
      wire Envoy xDS for transparent MITM header injection"
```

---

## Dependency Graph

```
Task 1 (fix secret table) ──┐
                             ├─→ Task 4 (boot flow) ──→ Task 7 (live e2e)
Task 2 (SecretEditor) ──────┤                                    │
                             │                                    ▼
Task 3 (BFF routes) ────────┘                            Task 8 (proxy spike)
                                                                  ▲
Task 5 (SDK snippet) ─── independent                              │
Task 6 (echo server) ────────────────────────────────────────────┘
```

Tasks 1, 2, 3, 5, 6 can run in parallel. Task 4 depends on 1+2+3. Task 7 depends on 4+6. Task 8 depends on 6+7.
