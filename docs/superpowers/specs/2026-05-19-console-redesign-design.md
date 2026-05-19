# Console Redesign — Developer-First Quickstart Experience

**Date:** 2026-05-19
**Status:** Approved design, pending implementation plan

## Problem

The console's sandboxes list page shows a flat inventory of running and terminated sandboxes with operational metadata (drone names, trust tiers, vCPU counts, ages). None of this helps a developer who needs to learn how to use sandboxes from code. The "Use in your app" SDK snippet exists but is buried behind a button click in two separate pages. The pages feel disconnected — no coherent journey from learning to building.

## Solution

Replace the sandboxes list with a quickstart guide. Restructure the console into three pages with a single flow: **Learn → Try → Build**.

## Pages

### 1. Quickstart (`/quickstart`, replaces `/sandboxes`)

The primary educational surface. Teaches developers how to use sandboxes from their code, with live sandboxes woven in as proof.

**Structure:**

```
Hero: "Run code in secure sandboxes"
       "Create, connect, and execute — from your app, in three lines."

Install: $ npm i @sandbox-platform/sdk   or   $ pip install sandbox-platform

Runtime switcher: [ Node.js ] [ Python ] [ Agent ] [ Desktop ]
(all code snippets on the page adapt when the developer switches runtime)

Step 1: Create a sandbox
  - Code snippet: Sandbox.create({ runtime, image, resources })
  - Desktop variant: same create call, different runtime/image values

Step 2: Use it
  - CLI runtimes: runCommand example with stdout capture
  - Desktop runtime: screenshot() example with VNC preview mock

Step 3: Interact with it
  - CLI: writeFile, readFile, install packages + run
  - Desktop: click(), type(), sendKeys(), screenshot()

Step 4: Connect to a running sandbox
  - Shows live running sandboxes (fetched from API, status=ready only)
  - Each shows: green dot, sandbox ID, runtime badge, "Open" and "Copy ID" buttons
  - Code snippet: Sandbox.get("<actual-sandbox-id>")
  - If no running sandboxes: "No sandboxes running. Boot one in the Playground →"
```

**Runtime switcher behavior:**
- Four buttons at the top: Node.js, Python, Agent, Desktop
- Node.js, Python, and Agent share identical step structure (CLI runtimes) — only the `runtime` and `image` values in the create snippet differ
- Desktop switches steps 2 and 3 to visual/automation APIs
- Step 4 always adapts to show the correct API for the runtime type of each live sandbox

**Language tabs within steps:**
- Each code snippet has TypeScript / Python tabs
- Language selection persists across steps (choosing Python in step 1 keeps Python selected in step 2)
- Language preference stored in localStorage

**Data requirements:**
- Fetch running sandboxes via existing `listSandboxes({ status: "ready" })` API
- No new API endpoints needed
- Polling: same 10s refetch interval as current list page

### 2. Try it live (on Quickstart, `/quickstart#try-live`)

Boot a sandbox from the quickstart page via the **Try it live** panel (runtime grid + config). On success, navigate to sandbox detail for the full terminal.

**Playground route (`/playground`):** redirects to `/quickstart#try-live` (deprecated as a separate surface).

### 3. Sandbox Detail (`/sandboxes/[id]`, enhanced)

The working surface for a specific sandbox.

**Changes from current:**
- SDK snippet panel **open by default** (currently hidden behind "Use in your app" button)
- Panel shows connection code for this specific sandbox, adapting to runtime type
- Breadcrumb changes from "Sandboxes /" to "Quickstart /" linking to `/quickstart`
- For desktop runtime: show VNC view instead of terminal (existing behavior, unchanged)
- Keep: terminal, execution history toggle, stop button, status badge

### Removed

- `/sandboxes` list page — replaced entirely by `/quickstart`
- The flat sandbox list component and its terminated-sandbox rendering
- "Sandboxes" nav item in topbar → replaced with "Quickstart"

### Navigation

Topbar shows one item:
- **Quickstart** (links to `/quickstart`; active on sandbox detail routes too)

`/` redirects to `/quickstart`. `/playground` redirects to `/quickstart#try-live`.

## Responsive Design

All three pages must work across desktop, tablet, and mobile breakpoints using the project's existing Tailwind responsive utilities.

### Breakpoint strategy

Follow the existing pattern in the codebase: mobile-first with `sm:` (640px) and up.

### Quickstart (`/quickstart`)

**Mobile (< 640px):**
- Install row stacks vertically (npm above pip, "or" between)
- Runtime switcher: buttons shrink to icon-only (N, P, D) without labels, arranged in a horizontal row
- Code blocks: full width, horizontal scroll for long lines
- Language tabs: same size, still inline
- Live sandbox cards: stack the action buttons below the sandbox info
- Steps: step number + vertical connector line remain, content fills available width

**Tablet / Desktop (≥ 640px):**
- Install row horizontal with "or" inline
- Runtime switcher: icon + label, centered
- Code blocks: comfortable max-width (520px centered), no horizontal scroll needed at normal snippet lengths
- Live sandbox cards: info left, buttons right in a single row

### Playground (`/playground`)

Already responsive (existing implementation uses `sm:` breakpoints throughout). No changes to responsive behavior.

### Sandbox Detail (`/sandboxes/[id]`)

**Mobile (< 640px):**
- SDK snippet panel renders **below** the terminal (stacked), not as a side panel
- Full-width terminal, full-width snippet
- Header bar: breadcrumb hidden, show only sandbox ID + status badge; action buttons collapse to icon-only
- Language tabs in snippet panel remain inline

**Tablet (640px–1024px):**
- SDK snippet panel as a narrower right panel (200px) alongside terminal
- Header bar: breadcrumb visible, buttons show labels

**Desktop (> 1024px):**
- SDK snippet panel at full 260px width alongside terminal
- Header bar: full breadcrumb, all labels visible

### Detail page snippet panel — default open behavior on mobile

On screens < 640px, the snippet panel defaults to **collapsed** (with a sticky "Use in code" button at the bottom of the screen to expand it). This avoids pushing the terminal off-screen on first load. On ≥ 640px, the panel is open by default as a side panel.

## Components

### New components

- **`RuntimeSwitcher`** — horizontal button group for selecting Node.js / Python / Desktop. Controls which code variant is shown across all steps. Props: `selected`, `onChange`.
- **`QuickstartStep`** — a numbered step with vertical connector line. Props: `number`, `title`, `description`, `children`.
- **`CodeSnippet`** — syntax-highlighted code block with language tabs (TS/Py) and copy button. Props: `snippets: Record<Language, string>`, `selectedLang`, `onLangChange`. Reuses existing `SdkSnippet` styling but decoupled from sandbox-specific generation logic.
- **`LiveSandboxCard`** — green-bordered card for a running sandbox with ID, runtime badge, Open/Copy ID buttons. Props: `sandbox: Sandbox`.
- **`InstallBanner`** — the npm/pip install row. Static content, responsive layout.

### Modified components

- **`SdkSnippet`** — no structural changes, but now rendered with `defaultOpen` behavior on the detail page. Add an `open` prop to control initial visibility.
- **`Topbar`** — nav items change from ["Playground", "Sandboxes"] to ["Quickstart", "Playground"]. Routes update accordingly.
- **Sandbox detail page** — snippet panel open by default (≥ 640px), breadcrumb links to `/quickstart`.

### Removed components

- **`SandboxRow`** — the list row component. No longer used once the list page is replaced.

### Preserved components (no changes)

- `TerminalView`, `ExecutionHistory`, `SandboxConfig`, `ColdStartTimer`, `Providers`

## Routes

| Route | Page | Change |
|---|---|---|
| `/` | Redirect to `/quickstart` | New (was Playground) |
| `/quickstart` | Quickstart guide | New (replaces `/sandboxes` list) |
| `/playground` | Playground (boot + try) | Moved from `/` |
| `/sandboxes/[id]` | Sandbox detail | Enhanced (snippet open by default) |
| `/sandboxes/[id]/terminal` | Terminal-only view | Unchanged |
| `/sandboxes` | — | Removed (redirects to `/quickstart`) |

API routes (`/api/sandboxes/*`) are unchanged.

## Code snippet content

The quickstart page has static code snippets (not dynamically generated from a live sandbox). The snippets are hardcoded in the page component, organized by runtime and language.

**CLI runtimes (Node.js, Python):**

Step 1 — Create:
```typescript
import { Sandbox } from "@sandbox-platform/sdk";

const sandbox = await Sandbox.create({
  runtime: "node24",
  image: "sandbox-runtime:node24-latest",
  resources: { vcpus: 2 },
});
```

Step 2 — Run code:
```typescript
const result = await sandbox.runCommand("/bin/bash", {
  args: ["-c", "node -e 'console.log(42)'"],
});
console.log(result.stdout); // "42\n"
```

Step 3 — Interact:
```typescript
await sandbox.writeFile("/app/index.js", code);
const contents = await sandbox.readFile("/app/index.js");
await sandbox.runCommand("/bin/bash", {
  args: ["-c", "cd /app && npm install && node index.js"],
});
```

**Desktop runtime:**

Step 1 — Create:
```typescript
import { Sandbox } from "@sandbox-platform/sdk";

const desktop = await Sandbox.create({
  runtime: "desktop",
  image: "sandbox-runtime:desktop-latest",
  resources: { vcpus: 4 },
});
```

Step 2 — Screenshot:
```typescript
const screenshot = await desktop.screenshot();
await fs.writeFile("screen.png", screenshot);
```

Step 3 — Automate:
```typescript
await desktop.click({ x: 512, y: 384 });
await desktop.type("Hello, world!");
await desktop.sendKeys("ctrl+s");
const after = await desktop.screenshot();
```

**Step 4 (connect)** is dynamic — it reads the actual sandbox ID from the live sandbox list and generates a `Sandbox.get("sb-...")` snippet.

Python variants mirror the TypeScript structure using the `sandbox_platform` package and Python syntax.

## Design tokens

All styling uses the project's existing design system defined in `globals.css` and `theme.ts`. No new tokens. Key values used:

- Backgrounds: `bg` (#000), `bg-1` (#0a0a0a), `bg-2` (#111)
- Borders: `border` (#222), `border-light` (#333)
- Text: `text` (#ededed), `text-2` (#a1a1a1), `text-3` (#666), `text-4` (#444)
- Status: `status-green` (#0cce6b)
- Accent: `accent-blue` (#0070f3), `accent-purple` (#7928ca)
- Font: `font-sans` (Inter), `font-mono` (SF Mono/Menlo)
- Radii: `radius-md` (6px), `radius-lg` (8px), `radius-xl` (12px)

## Testing

- **Quickstart page:** renders all 4 steps, runtime switcher toggles content, language tabs persist, live sandboxes appear when API returns ready sandboxes, empty state shows "no sandboxes running" message
- **Runtime switcher:** clicking Desktop hides CLI content and shows desktop content, clicking Node/Python restores CLI content
- **LiveSandboxCard:** renders sandbox ID, runtime badge, Open button links to detail page, Copy ID copies to clipboard
- **CodeSnippet:** language tab switching, copy button, syntax highlighting renders
- **Responsive:** on viewport < 640px, install row stacks, runtime buttons show icon-only, detail page snippet panel collapses below terminal
- **Navigation:** topbar shows Quickstart and Playground, active state correct, `/` redirects to `/quickstart`, `/sandboxes` redirects to `/quickstart`
- **Detail page:** snippet panel open by default on desktop, collapsed on mobile with expand button
