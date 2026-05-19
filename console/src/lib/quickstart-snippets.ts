import type { SnippetLanguage } from "./use-persisted-language";

/** Quickstart environment: headless CLI vs desktop automation. */
export type QuickstartEnv = "cli" | "desktop";

export const ENV_OPTIONS = [
  { id: "cli" as const, label: "CLI", icon: "C" },
  { id: "desktop" as const, label: "Desktop", icon: "D" },
] as const;

const CLI_RUNTIME_META = {
  typescript: { runtime: "node24", image: "sandbox-runtime:node24-latest" },
  python: { runtime: "python313", image: "sandbox-runtime:python313-latest" },
} as const;

function cliStep1Ts(lang: SnippetLanguage): string {
  const { runtime, image } = CLI_RUNTIME_META[lang];
  return `import { Sandbox } from "@sandbox-platform/sdk";

const sandbox = await Sandbox.create({
  runtime: "${runtime}",
  image: "${image}",
  resources: { vcpus: 2 },
});`;
}

function cliStep1Py(lang: SnippetLanguage): string {
  const { runtime, image } = CLI_RUNTIME_META[lang];
  return `from sandbox_platform import Sandbox, CreateSandboxRequest, ResourcesSpec

sandbox = Sandbox.create(CreateSandboxRequest(
    runtime="${runtime}",
    image="${image}",
    resources=ResourcesSpec(vcpus=2),
))`;
}

const CLI_STEP2_TS = `const result = await sandbox.runCommand("/bin/bash", {
  args: ["-c", "node -e 'console.log(42)'"],
});
console.log(result.stdout); // "42\\n"`;

const CLI_STEP2_PY = `result = sandbox.run_command("/bin/bash", args=["-c", "node -e 'console.log(42)'"])
print(result["stdout"])  # "42\\n"`;

const CLI_STEP3_TS = `await sandbox.writeFile("/app/index.js", code);
const contents = await sandbox.readTextFile("/app/index.js");
await sandbox.runCommand("/bin/bash", {
  args: ["-c", "cd /app && npm install && node index.js"],
});`;

const CLI_STEP3_PY = `sandbox.write_file("/app/index.js", code)
contents = sandbox.read_text_file("/app/index.js")
sandbox.run_command("/bin/bash", args=["-c", "cd /app && npm install && node index.js"])`;

const DESKTOP_STEP1_TS = `import { Sandbox } from "@sandbox-platform/sdk";

const desktop = await Sandbox.create({
  runtime: "desktop",
  image: "sandbox-runtime:desktop-latest",
  resources: { vcpus: 4 },
});`;

const DESKTOP_STEP1_PY = `from sandbox_platform import Sandbox, CreateSandboxRequest, ResourcesSpec

desktop = Sandbox.create(CreateSandboxRequest(
    runtime="desktop",
    image="sandbox-runtime:desktop-latest",
    resources=ResourcesSpec(vcpus=4),
))`;

const DESKTOP_STEP2_TS = `const screenshot = await desktop.screenshot();
await fs.writeFile("screen.png", screenshot);`;

const DESKTOP_STEP2_PY = `screenshot = desktop.screenshot()
with open("screen.png", "wb") as f:
    f.write(screenshot)`;

const DESKTOP_STEP3_TS = `await desktop.click([512, 384]);
await desktop.type("Hello, world!");
await desktop.sendKeys("ctrl+s");
const after = await desktop.screenshot();`;

const DESKTOP_STEP3_PY = `desktop.click(x=512, y=384)
desktop.type_text("Hello, world!")
desktop.send_keys("ctrl+s")
after = desktop.screenshot()`;

export function getStepSnippet(
  step: 1 | 2 | 3,
  env: QuickstartEnv,
  lang: SnippetLanguage,
): string {
  const isDesktop = env === "desktop";

  if (step === 1) {
    if (isDesktop) return lang === "typescript" ? DESKTOP_STEP1_TS : DESKTOP_STEP1_PY;
    return lang === "typescript" ? cliStep1Ts(lang) : cliStep1Py(lang);
  }
  if (step === 2) {
    if (isDesktop) return lang === "typescript" ? DESKTOP_STEP2_TS : DESKTOP_STEP2_PY;
    return lang === "typescript" ? CLI_STEP2_TS : CLI_STEP2_PY;
  }
  if (isDesktop) return lang === "typescript" ? DESKTOP_STEP3_TS : DESKTOP_STEP3_PY;
  return lang === "typescript" ? CLI_STEP3_TS : CLI_STEP3_PY;
}

export function connectSnippet(sandboxId: string, lang: SnippetLanguage): string {
  if (lang === "typescript") {
    return `import { Sandbox } from "@sandbox-platform/sdk";

const sandbox = await Sandbox.get("${sandboxId}");`;
  }
  return `from sandbox_platform import Sandbox

sandbox = Sandbox.get("${sandboxId}")`;
}
