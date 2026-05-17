import { createHttpClient, type HttpClient } from "./http.js";
import { SandboxError, TimeoutError } from "./errors.js";
import { normalizeBaseUrl, sleep } from "./utils.js";
import type {
  ConnectResponse,
  DirEntry,
  ExecSyncResponse,
  ExecDetachedResponse,
  ListeningPort,
  ListResponse,
  MoveRequest,
  SignalName,
} from "./generated/types.js";

export type SandboxOptions = {
  controlUrl: string;
  token?: string;
};

export type CreateOptions = {
  runtime: string;
  image?: string;
  cluster?: string;
  pool?: string;
  env?: Record<string, string>;
  timeout_ms?: number;
  use_static_token?: boolean;
  key?: { name: string; namespace?: string; tag?: string };
};

export type RunCommandOptions = {
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  stdin?: string;
  timeout_ms?: number;
  user?: string;
};

export type RunCommandResult = {
  cmdId: string;
  exitCode: number;
  stdout: string;
  stderr: string;
  durationMs: number;
  truncated: boolean;
};

export type SpawnOptions = RunCommandOptions & {
  pty?: boolean;
  ptySize?: { rows: number; cols: number };
};

export type SpawnResult = {
  cmdId: string;
};

export type WriteFileOptions = {
  mode?: string;
};

export type WatchOptions = {
  recursive?: boolean;
  signal?: AbortSignal;
};

const RUNTIME_TO_IMAGE: Record<string, string> = {
  node22: "node:22-slim",
  node24: "node:24-slim",
  python313: "python:3.13-slim",
  bun1: "oven/bun:1",
  desktop: "sandbox-platform/desktop:latest",
};

export class Sandbox {
  protected readonly dataPlaneClient: HttpClient;
  protected readonly baseUrl: string;
  readonly backendId: string;
  readonly url: string;
  readonly statusUrl: string;
  readonly secretToken: string | undefined;

  protected constructor(connectResponse: ConnectResponse, baseUrl: string) {
    this.backendId = connectResponse.backend_id;
    this.url = connectResponse.url;
    this.statusUrl = connectResponse.status_url;
    this.secretToken = connectResponse.secret_token;
    this.dataPlaneClient = createHttpClient(baseUrl);
    this.baseUrl = normalizeBaseUrl(baseUrl);
  }

  static async create(
    sandboxOpts: SandboxOptions,
    createOpts: CreateOptions,
  ): Promise<Sandbox> {
    const controlClient = createHttpClient(
      sandboxOpts.controlUrl,
      sandboxOpts.token,
    );

    const executable: Record<string, unknown> = {
      image: createOpts.image ?? runtimeToImage(createOpts.runtime),
      env: createOpts.env ?? {},
    };

    const connectResponse = await controlClient.request<ConnectResponse>({
      method: "POST",
      path: "/ctrl/connect",
      body: {
        spawn_config: {
          cluster: createOpts.cluster,
          pool: createOpts.pool ?? "untrusted",
          executable,
          lifetime_limit_seconds: createOpts.timeout_ms
            ? Math.ceil(createOpts.timeout_ms / 1000)
            : undefined,
          use_static_token: createOpts.use_static_token,
        },
        key: createOpts.key,
      },
    });

    return new Sandbox(connectResponse, connectResponse.url);
  }

  static fromConnection(
    url: string,
    connectResponse: ConnectResponse,
  ): Sandbox {
    return new Sandbox(connectResponse, url);
  }

  async runCommand(
    cmd: string,
    options: RunCommandOptions = {},
  ): Promise<RunCommandResult> {
    const body = buildExecBody(cmd, false, options);

    const response =
      await this.dataPlaneClient.request<ExecSyncResponse>({
        method: "POST",
        path: "/v1/exec",
        body,
      });

    return {
      cmdId: response.cmd_id,
      exitCode: response.exit_code,
      stdout: response.stdout,
      stderr: response.stderr,
      durationMs: response.duration_ms,
      truncated: response.truncated ?? false,
    };
  }

  async spawn(cmd: string, options: SpawnOptions = {}): Promise<SpawnResult> {
    const body = buildExecBody(cmd, true, options);
    if (options.pty) body.pty = options.pty;
    if (options.ptySize) body.pty_size = options.ptySize;

    const response =
      await this.dataPlaneClient.request<ExecDetachedResponse>({
        method: "POST",
        path: "/v1/exec",
        body,
      });

    return { cmdId: response.cmd_id };
  }

  async getProcessStatus(
    cmdId: string,
  ): Promise<{ status: "running" | "exited"; exitCode?: number }> {
    const response = await this.dataPlaneClient.request<{
      cmd_id: string;
      status: "running" | "exited";
      exit_code?: number | null;
    }>({
      method: "GET",
      path: `/v1/exec/${encodeURIComponent(cmdId)}`,
    });

    const result: { status: "running" | "exited"; exitCode?: number } = {
      status: response.status,
    };
    if (response.exit_code != null) {
      result.exitCode = response.exit_code;
    }
    return result;
  }

  async writeStdin(cmdId: string, data: string): Promise<void> {
    await this.dataPlaneClient.request<unknown>({
      method: "POST",
      path: `/v1/exec/${encodeURIComponent(cmdId)}/stdin`,
      body: { data },
    });
  }

  async signal(cmdId: string, signal: SignalName): Promise<void> {
    await this.dataPlaneClient.requestRaw({
      method: "POST",
      path: `/v1/exec/${encodeURIComponent(cmdId)}/signal`,
      body: { signal },
    });
  }

  async readFile(path: string): Promise<Uint8Array> {
    const response = await this.dataPlaneClient.requestRaw({
      method: "GET",
      path: "/v1/fs/read",
      query: { path },
    });
    const buffer = await response.arrayBuffer();
    return new Uint8Array(buffer);
  }

  async readTextFile(path: string): Promise<string> {
    const response = await this.dataPlaneClient.requestRaw({
      method: "GET",
      path: "/v1/fs/read",
      query: { path },
    });
    return response.text();
  }

  async writeFile(
    path: string,
    content: string | Uint8Array,
    options: WriteFileOptions = {},
  ): Promise<void> {
    const blob =
      typeof content === "string"
        ? new Blob([content], { type: "text/plain" })
        : new Blob([content], { type: "application/octet-stream" });

    const formData = new FormData();
    formData.append("path", path);
    if (options.mode) {
      formData.append("mode", options.mode);
    }
    formData.append("file", blob, "file");

    const url = `${this.baseUrl}/v1/fs/write`;
    const response = await fetch(url, {
      method: "PUT",
      body: formData,
    });

    if (!response.ok) {
      const text = await response.text().catch(() => "");
      throw new SandboxError(
        `writeFile failed: HTTP ${response.status}: ${text}`,
        response.status,
      );
    }
  }

  async mkdir(path: string, options: { recursive?: boolean } = {}): Promise<void> {
    const body: Record<string, unknown> = { path };
    if (options.recursive !== undefined) {
      body.recursive = options.recursive;
    }
    await this.dataPlaneClient.requestRaw({
      method: "POST",
      path: "/v1/fs/mkdir",
      body,
    });
  }

  async rm(path: string, options: { recursive?: boolean } = {}): Promise<void> {
    await this.dataPlaneClient.requestRaw({
      method: "DELETE",
      path: "/v1/fs/rm",
      query: {
        path,
        ...(options.recursive !== undefined && {
          recursive: String(options.recursive),
        }),
      },
    });
  }

  async listDir(path: string): Promise<DirEntry[]> {
    const response = await this.dataPlaneClient.request<ListResponse>({
      method: "GET",
      path: "/v1/fs/list",
      query: { path },
    });
    return response.entries;
  }

  async move(source: string, destination: string): Promise<void> {
    const body: MoveRequest = { source, destination };
    await this.dataPlaneClient.requestRaw({
      method: "POST",
      path: "/v1/fs/move",
      body,
    });
  }

  async listPorts(): Promise<ListeningPort[]> {
    const response = await this.dataPlaneClient.request<{
      listening: ListeningPort[];
    }>({
      method: "GET",
      path: "/v1/ports",
    });
    return response.listening;
  }

  async waitForPort(
    port: number,
    timeoutMs: number = 30_000,
  ): Promise<void> {
    const deadline = Date.now() + timeoutMs;
    const pollInterval = 500;

    while (Date.now() < deadline) {
      const ports = await this.listPorts();
      if (ports.some((p) => p.port === port)) {
        return;
      }
      await sleep(pollInterval);
    }

    throw new TimeoutError(
      `port ${port} did not become available within ${timeoutMs}ms`,
    );
  }
}

function buildExecBody(
  cmd: string,
  detached: boolean,
  options: RunCommandOptions,
): Record<string, unknown> {
  const body: Record<string, unknown> = { cmd, detached };
  if (options.args) body.args = options.args;
  if (options.env) body.env = options.env;
  if (options.cwd) body.cwd = options.cwd;
  if (options.stdin) body.stdin = options.stdin;
  if (options.timeout_ms) body.timeout_ms = options.timeout_ms;
  if (options.user) body.user = options.user;
  return body;
}

function runtimeToImage(runtime: string): string {
  return RUNTIME_TO_IMAGE[runtime] ?? runtime;
}
