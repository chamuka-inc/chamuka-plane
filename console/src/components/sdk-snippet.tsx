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

function deriveEnvVarName(destination: string): string {
  const parts = destination.replace(/^api\./, "").split(".");
  const base = parts[0].toUpperCase().replace(/[^A-Z0-9]/g, "_");
  return `${base}_API_KEY`;
}

function varName(destination: string): string {
  const parts = destination.replace(/^api\./, "").split(".");
  return parts[0].replace(/[^a-zA-Z0-9]/g, "").toLowerCase();
}

function hasConfigOverrides(c?: SandboxConfigValues): boolean {
  if (!c) return false;
  return (
    c.trust !== DEFAULT_CONFIG.trust ||
    c.vcpus !== DEFAULT_CONFIG.vcpus ||
    c.timeout_ms !== DEFAULT_CONFIG.timeout_ms ||
    Object.keys(c.env).length > 0 ||
    (c.secrets?.length ?? 0) > 0 ||
    c.persistent
  );
}

function generateTs(props: SdkSnippetProps): string {
  const c = props.config;
  const hasOverrides = hasConfigOverrides(c);
  const hasSecrets = (c?.secrets?.length ?? 0) > 0;

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
    hasSecrets
      ? `import { Sandbox, SandboxClient } from "@sandbox-platform/sdk";`
      : `import { Sandbox } from "@sandbox-platform/sdk";`,
    ``,
  ];

  if (hasSecrets) {
    lines.push(`const client = new SandboxClient({ token: process.env.SANDBOX_TOKEN! });`);
    lines.push(``);
  }

  lines.push(
    `const sandbox = await Sandbox.create({`,
    ...createArgs,
    `});`,
  );

  if (c?.secrets?.length) {
    lines.push(``);
    for (const s of c.secrets) {
      const envVar = deriveEnvVarName(s.destination);
      const vn = varName(s.destination);
      lines.push(`const { id: ${vn}SecretId } = await client.createSecret({`);
      lines.push(`  value: process.env.${envVar}!,`);
      if (s.destination) lines.push(`  name: ${JSON.stringify(s.destination)},`);
      lines.push(`});`);
      lines.push(`await sandbox.createCredential({`);
      lines.push(`  secret_id: ${vn}SecretId,`);
      lines.push(`  destination: ${JSON.stringify(s.destination)},`);
      lines.push(`  header_name: ${JSON.stringify(s.header_name)},`);
      if (s.header_format !== "{value}") {
        lines.push(`  header_format: ${JSON.stringify(s.header_format)},`);
      }
      lines.push(`});`);
    }
  }

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
  const hasSecrets = (c?.secrets?.length ?? 0) > 0;

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

  const imports = [
    hasSecrets
      ? `from sandbox_platform import Sandbox, SandboxClient, CreateSandboxRequest, ResourcesSpec`
      : `from sandbox_platform import Sandbox, CreateSandboxRequest, ResourcesSpec`,
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

  lines.push(`sandbox = Sandbox.create(CreateSandboxRequest(`);

  for (const kw of kwargs) {
    lines.push(`    ${kw},`);
  }
  lines.push(`))`);

  if (c?.secrets?.length) {
    lines.push(``);
    for (const s of c.secrets) {
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
        <code role="code">{code}</code>
      </pre>
    </div>
  );
}
