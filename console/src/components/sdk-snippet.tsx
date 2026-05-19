"use client";

import { useState, useCallback } from "react";
import { Copy, Check } from "lucide-react";

type Language = "typescript" | "python";

import type { SandboxConfigValues } from "./sandbox-config";
import { DEFAULT_CONFIG } from "./sandbox-config";

type SdkSnippetProps = {
  runtime: string;
  image: string;
  sandboxId?: string;
  commands?: string[];
  lastCommand?: string;
  config?: SandboxConfigValues;
};

function hasConfigOverrides(c?: SandboxConfigValues): boolean {
  if (!c) return false;
  return (
    c.trust !== DEFAULT_CONFIG.trust ||
    c.vcpus !== DEFAULT_CONFIG.vcpus ||
    c.timeout_ms !== DEFAULT_CONFIG.timeout_ms ||
    Object.keys(c.env).length > 0 ||
    c.persistent
  );
}

function generateTs(props: SdkSnippetProps): string {
  const c = props.config;
  const hasOverrides = hasConfigOverrides(c);

  const vcpus = c?.vcpus ?? DEFAULT_CONFIG.vcpus;
  const createArgs: string[] = [
    `  runtime: "${props.runtime}",`,
    `  image: "${props.image}",`,
    `  resources: { vcpus: ${vcpus} },`,
  ];

  if (c && hasOverrides) {
    if (c.trust !== DEFAULT_CONFIG.trust) {
      createArgs.push(`  trust: "${c.trust}",`);
    }
    if (c.timeout_ms !== DEFAULT_CONFIG.timeout_ms) {
      createArgs.push(`  timeout_ms: ${c.timeout_ms},`);
    }
    if (c.persistent) {
      createArgs.push(`  persistent: true,`);
    }
    const envKeys = Object.keys(c.env);
    if (envKeys.length > 0) {
      createArgs.push(`  env: {`);
      for (const [k, v] of Object.entries(c.env)) {
        createArgs.push(`    ${JSON.stringify(k)}: ${JSON.stringify(v)},`);
      }
      createArgs.push(`  },`);
    }
  }

  const lines = [
    `import { Sandbox } from "@sandbox-platform/sdk";`,
    ``,
    `const sandbox = await Sandbox.create({`,
    ...createArgs,
    `});`,
  ];

  const cmds = props.commands?.length ? props.commands : props.lastCommand ? [props.lastCommand] : [];

  if (cmds.length === 1) {
    lines.push(
      ``,
      `const result = await sandbox.runCommand("/bin/bash", {`,
      `  args: ["-c", ${JSON.stringify(cmds[0])}],`,
      `});`,
      `console.log(result.stdout);`,
    );
  } else if (cmds.length > 1) {
    lines.push(``);
    for (const cmd of cmds) {
      lines.push(`await sandbox.runCommand("/bin/bash", { args: ["-c", ${JSON.stringify(cmd)}] });`);
    }
    lines.push(``);
    lines.push(`// Or capture output:`);
    lines.push(`// const result = await sandbox.runCommand("/bin/bash", { args: ["-c", "..."] });`);
    lines.push(`// console.log(result.stdout);`);
  }

  return lines.join("\n");
}

function generatePy(props: SdkSnippetProps): string {
  const c = props.config;
  const hasOverrides = hasConfigOverrides(c);

  const kwargs: string[] = [
    `runtime="${props.runtime}"`,
    `image="${props.image}"`,
    `resources=ResourcesSpec(vcpus=${c?.vcpus ?? 2})`,
  ];

  if (c && hasOverrides) {
    if (c.trust !== DEFAULT_CONFIG.trust) kwargs.push(`trust="${c.trust}"`);
    if (c.timeout_ms !== DEFAULT_CONFIG.timeout_ms) kwargs.push(`timeout_ms=${c.timeout_ms}`);
    if (c.persistent) kwargs.push(`persistent=True`);
    const envKeys = Object.keys(c.env);
    if (envKeys.length > 0) {
      const pairs = Object.entries(c.env).map(([k, v]) => `${JSON.stringify(k)}: ${JSON.stringify(v)}`).join(", ");
      kwargs.push(`env={${pairs}}`);
    }
  }

  const lines = [
    `from sandbox_platform import Sandbox, CreateSandboxRequest, ResourcesSpec`,
    ``,
    `sandbox = Sandbox.create(CreateSandboxRequest(`,
  ];

  for (const kw of kwargs) {
    lines.push(`    ${kw},`);
  }
  lines.push(`))`);

  appendPyExecs(lines, props);
  return lines.join("\n");
}

function appendPyExecs(lines: string[], props: SdkSnippetProps) {
  const cmds = props.commands?.length ? props.commands : props.lastCommand ? [props.lastCommand] : [];

  if (cmds.length === 1) {
    lines.push(
      ``,
      `result = sandbox.run_command("/bin/bash", args=["-c", ${JSON.stringify(cmds[0])}])`,
      `print(result["stdout"])`,
    );
  } else if (cmds.length > 1) {
    lines.push(``);
    for (const cmd of cmds) {
      lines.push(`sandbox.run_command("/bin/bash", args=["-c", ${JSON.stringify(cmd)}])`);
    }
    lines.push(``, `# Or capture output:`, `# result = sandbox.run_command("/bin/bash", args=["-c", "..."])`, `# print(result["stdout"])`);
  }
}

export function SdkSnippet(props: SdkSnippetProps) {
  const [lang, setLang] = useState<Language>("typescript");
  const [copied, setCopied] = useState(false);

  const code = lang === "typescript" ? generateTs(props) : generatePy(props);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [code]);

  return (
    <div className="border border-border rounded-lg overflow-hidden bg-bg-1">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <div className="flex gap-1">
          <button
            onClick={() => setLang("typescript")}
            className={`text-xs px-2 py-1 rounded-md transition-colors ${
              lang === "typescript"
                ? "bg-bg-2 text-text"
                : "text-text-3 hover:text-text"
            }`}
          >
            TypeScript
          </button>
          <button
            onClick={() => setLang("python")}
            className={`text-xs px-2 py-1 rounded-md transition-colors ${
              lang === "python"
                ? "bg-bg-2 text-text"
                : "text-text-3 hover:text-text"
            }`}
          >
            Python
          </button>
        </div>
        <button
          onClick={handleCopy}
          className="text-text-3 hover:text-text transition-colors p-1"
          aria-label="Copy code"
        >
          {copied ? <Check size={14} className="text-status-green" /> : <Copy size={14} />}
        </button>
      </div>
      <pre className="p-3 text-xs leading-relaxed overflow-x-auto font-mono text-text-2">
        <code>{code}</code>
      </pre>
    </div>
  );
}
