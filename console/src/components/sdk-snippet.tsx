"use client";

import { useState } from "react";
import type { SandboxConfigValues } from "./sandbox-config";
import { DEFAULT_CONFIG } from "./sandbox-config";
import { CodeSnippet } from "./code-snippet";
import { connectSnippet } from "@/lib/quickstart-snippets";
import type { SnippetLanguage } from "@/lib/use-persisted-language";

export type SdkSnippetProps = {
  runtime: string;
  image: string;
  sandboxId?: string;
  commands?: string[];
  lastCommand?: string;
  config?: SandboxConfigValues;
  variant?: "full" | "connect";
  language?: SnippetLanguage;
  onLanguageChange?: (lang: SnippetLanguage) => void;
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
  ];

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
      lines.push(`const { id: ${vn}SecretId } = await Sandbox.createSecret({`);
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
    `from sandbox_platform import Sandbox, CreateSandboxRequest, ResourcesSpec`,
  ];
  if (hasSecrets) {
    imports.push(`from sandbox_platform import CreateSecretRequest, CreateCredentialBindingRequest`);
    imports.push(`import os`);
  }

  const lines = [...imports, ``];

  lines.push(`sandbox = Sandbox.create(CreateSandboxRequest(`);

  for (const kw of kwargs) {
    lines.push(`    ${kw},`);
  }
  lines.push(`))`);

  if (c?.secrets?.length) {
    lines.push(``);
    for (const s of c.secrets) {
      const envVar = deriveEnvVarName(s.destination);
      lines.push(`secret = Sandbox.create_secret(CreateSecretRequest(`);
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

function resolveCode(props: SdkSnippetProps, lang: SnippetLanguage): string {
  if (props.variant === "connect" && props.sandboxId) {
    return connectSnippet(props.sandboxId, lang);
  }
  return lang === "typescript" ? generateTs(props) : generatePy(props);
}

export function SdkSnippet(props: SdkSnippetProps) {
  const [internalLang, setInternalLang] = useState<SnippetLanguage>("typescript");
  const language = props.language ?? internalLang;
  const onLanguageChange = props.onLanguageChange ?? setInternalLang;

  const code = resolveCode(props, language);

  return (
    <CodeSnippet
      code={code}
      language={language}
      onLanguageChange={onLanguageChange}
    />
  );
}
