import { createHttpClient, type HttpClient } from "./http.js";
import { SandboxError, TimeoutError } from "./errors.js";
import { normalizeBaseUrl, sleep } from "./utils.js";
import { resolveAuth, type SandboxAuth } from "./auth.js";
import { SandboxClient, type SnapshotOptions, type StopOptions, type ListOptions } from "./client.js";
import type {
  ComputerActionRequest,
  ComputerOutput,
  CreateSandboxRequest,
  CreateSecretRequest,
  CreateSecretResponse,
  DirEntry,
  ExecSyncResponse,
  ExecDetachedResponse,
  ListSandboxesResponse,
  ListeningPort,
  ListResponse,
  NetworkPolicy,
  CredentialBinding,
  CreateCredentialBindingRequest,
  Signal,
  Sandbox as SandboxType,
  SnapshotResponse,
} from "./generated/types.js";

export type RunCommandOptions = {
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  stdin?: string;
  timeout_ms?: number;
  user?: string;
  detached?: boolean;
  pty?: boolean;
  ptySize?: { rows: number; cols: number };
};

export type RunCommandResult = {
  cmdId: string;
  exitCode: number;
  stdout: string;
  stderr: string;
  durationMs: number;
  truncated: boolean;
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

export type SandboxConnectionInfo = {
  /**
   * Plane backend currently hosting this sandbox. Null once the backend has
   * been reaped past retention — the sandbox row outlives its backend.
   * Operations requiring drone dispatch (terminate, snapshot, etc.) return
   * 410 Gone when this is null.
   */
  backend_id: string | null;
  url: string;
  status_url: string;
  secret_token?: string;
};

export class Sandbox {
  protected readonly dataPlaneClient: HttpClient;
  protected readonly baseUrl: string;
  readonly backendId: string | null;
  readonly url: string;
  readonly statusUrl: string;
  readonly secretToken: string | undefined;

  private _client?: SandboxClient;
  private _sandbox?: SandboxType;

  get id(): string | undefined {
    return this._sandbox?.id;
  }

  get status(): string | undefined {
    return this._sandbox?.status;
  }

  get runtime(): string | undefined {
    return this._sandbox?.runtime;
  }

  constructor(connection: SandboxConnectionInfo, baseUrl: string) {
    this.backendId = connection.backend_id ?? null;
    this.url = connection.url;
    this.statusUrl = connection.status_url;
    this.secretToken = connection.secret_token;
    this.dataPlaneClient = createHttpClient(baseUrl);
    this.baseUrl = normalizeBaseUrl(baseUrl);
  }

  // --- Static factory methods ---

  static async create(
    request: CreateSandboxRequest,
    auth?: SandboxAuth,
  ): Promise<Sandbox> {
    const resolved = resolveAuth(auth);
    const client = new SandboxClient(resolved);
    const response = await client.create(request);
    const connectionInfo: SandboxConnectionInfo = {
      backend_id: response.sandbox.backend_id ?? null,
      url: response.connection.url,
      status_url: response.connection.status_url,
    };
    if (response.connection.secret_token != null) {
      connectionInfo.secret_token = response.connection.secret_token;
    }
    const sandbox = new Sandbox(connectionInfo, response.connection.url);
    sandbox._client = client;
    sandbox._sandbox = response.sandbox;
    return sandbox;
  }

  static async get(
    sandboxId: string,
    auth?: SandboxAuth,
  ): Promise<Sandbox> {
    const resolved = resolveAuth(auth);
    const client = new SandboxClient(resolved);
    const response = await client.getWithConnection(sandboxId);
    const connection = response.connection;

    if (!connection) {
      const sandbox = new Sandbox(
        {
          backend_id: response.sandbox.backend_id ?? null,
          url: "",
          status_url: "",
        },
        "",
      );
      sandbox._client = client;
      sandbox._sandbox = response.sandbox;
      return sandbox;
    }

    const connectionInfo: SandboxConnectionInfo = {
      backend_id: response.sandbox.backend_id ?? null,
      url: connection.url,
      status_url: connection.status_url,
    };
    if (connection.secret_token != null) {
      connectionInfo.secret_token = connection.secret_token;
    }
    const sandbox = new Sandbox(connectionInfo, connection.url);
    sandbox._client = client;
    sandbox._sandbox = response.sandbox;
    return sandbox;
  }

  static async list(
    options?: ListOptions,
    auth?: SandboxAuth,
  ): Promise<ListSandboxesResponse> {
    const resolved = resolveAuth(auth);
    const client = new SandboxClient(resolved);
    return client.list(options);
  }

  // --- Command execution ---

  async runCommand(
    cmd: string,
    options: RunCommandOptions & { detached: true },
  ): Promise<SpawnResult>;
  async runCommand(
    cmd: string,
    options?: RunCommandOptions,
  ): Promise<RunCommandResult>;
  async runCommand(
    cmd: string,
    options: RunCommandOptions = {},
  ): Promise<RunCommandResult | SpawnResult> {
    if (options.detached) {
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
    await this.dataPlaneClient.requestRaw({
      method: "POST",
      path: `/v1/exec/${encodeURIComponent(cmdId)}/stdin`,
      body: { data },
    });
  }

  async signal(cmdId: string, signal: Signal): Promise<void> {
    await this.dataPlaneClient.requestRaw({
      method: "POST",
      path: `/v1/exec/${encodeURIComponent(cmdId)}/signal`,
      body: { signal },
    });
  }

  // --- Filesystem ---

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
    const body = { source, destination };
    await this.dataPlaneClient.requestRaw({
      method: "POST",
      path: "/v1/fs/move",
      body,
    });
  }

  // --- Ports ---

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

  // --- Control-plane delegation ---

  private requireClient(method: string): SandboxClient {
    if (!this._client) {
      throw new SandboxError(
        `${method}() requires a managed sandbox. Use Sandbox.create() or Sandbox.get() instead of the constructor.`,
      );
    }
    return this._client;
  }

  private requireId(method: string): string {
    const id = this._sandbox?.id;
    if (!id) {
      throw new SandboxError(
        `${method}() requires a managed sandbox with an ID.`,
      );
    }
    return id;
  }

  async snapshot(options: SnapshotOptions = {}): Promise<SnapshotResponse> {
    const client = this.requireClient("snapshot");
    const id = this.requireId("snapshot");
    return client.snapshot(id, options);
  }

  async setNetworkPolicy(policy: NetworkPolicy): Promise<NetworkPolicy> {
    const client = this.requireClient("setNetworkPolicy");
    const id = this.requireId("setNetworkPolicy");
    return client.setNetworkPolicy(id, policy);
  }

  async createCredential(
    binding: CreateCredentialBindingRequest,
  ): Promise<CredentialBinding> {
    const client = this.requireClient("createCredential");
    const id = this.requireId("createCredential");
    return client.createCredential(id, binding);
  }

  async deleteCredential(credentialId: string): Promise<void> {
    const client = this.requireClient("deleteCredential");
    const id = this.requireId("deleteCredential");
    return client.deleteCredential(id, credentialId);
  }

  async extend(additionalMs: number): Promise<SandboxType> {
    const client = this.requireClient("extend");
    const id = this.requireId("extend");
    return client.extend(id, additionalMs);
  }

  async stop(options: StopOptions = {}): Promise<void> {
    const client = this.requireClient("stop");
    const id = this.requireId("stop");
    return client.stop(id, options);
  }

  // --- Secrets (control-plane) ---

  static async createSecret(
    request: CreateSecretRequest,
    auth?: SandboxAuth,
  ): Promise<CreateSecretResponse> {
    const resolved = resolveAuth(auth);
    const client = new SandboxClient(resolved);
    return client.createSecret(request);
  }

  // --- Computer use (desktop variant) ---

  async computer(
    request: ComputerActionRequest,
  ): Promise<ComputerOutput> {
    const response = await this.dataPlaneClient.request<{
      output: ComputerOutput;
    }>({
      method: "POST",
      path: "/v1/computer",
      body: request,
    });
    return response.output;
  }

  async screenshot(): Promise<Uint8Array> {
    const output = await this.computer({ action: "screenshot" });
    if (output.type !== "image") {
      throw new SandboxError("screenshot returned non-image output");
    }
    return new Uint8Array(Buffer.from(output.base64, "base64"));
  }

  async click(coordinate: [number, number]): Promise<void> {
    await this.computer({ action: "left_click", coordinate });
  }

  async type(text: string): Promise<void> {
    await this.computer({ action: "type", text });
  }

  async sendKeys(keys: string): Promise<void> {
    await this.computer({ action: "key", text: keys });
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
