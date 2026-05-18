import { createHttpClient, type HttpClient } from "./http.js";
import { ManagedSandbox } from "./managed-sandbox.js";
import type {
  Sandbox,
  CreateSandboxRequest,
  CreateSandboxResponse,
  ListSandboxesResponse,
  SnapshotResponse,
  NetworkPolicy,
  CredentialBinding,
  CreateCredentialBindingRequest,
  CreateSecretRequest,
  CreateSecretResponse,
  Snapshot,
  ExtendTimeoutRequest,
  BackendStatus,
} from "./generated/types.js";

export type ClientOptions = {
  controlUrl: string;
  token?: string;
  tenant?: string;
};

export type ListOptions = {
  status?: BackendStatus;
  limit?: number;
  cursor?: string;
  tag?: string;
};

export type SnapshotOptions = {
  tag?: string;
  ttl_days?: number;
  tags?: Record<string, string>;
};

export type StopOptions = {
  force?: boolean;
};

export class SandboxClient {
  private readonly http: HttpClient;

  constructor(options: ClientOptions) {
    this.http = createHttpClient(options.controlUrl, options.token, options.tenant);
  }

  async create(request: CreateSandboxRequest): Promise<CreateSandboxResponse> {
    return this.http.request<CreateSandboxResponse>({
      method: "POST",
      path: "/pub/v1/sandboxes",
      body: request,
    });
  }

  async createSandbox(request: CreateSandboxRequest): Promise<ManagedSandbox> {
    const response = await this.create(request);
    return new ManagedSandbox(response.sandbox, response.connection, this);
  }

  async list(options: ListOptions = {}): Promise<ListSandboxesResponse> {
    const query: Record<string, string> = {};
    if (options.status) query.status = options.status;
    if (options.limit !== undefined) query.limit = String(options.limit);
    if (options.cursor) query.cursor = options.cursor;
    if (options.tag) query.tag = options.tag;

    return this.http.request<ListSandboxesResponse>({
      method: "GET",
      path: "/pub/v1/sandboxes",
      query,
    });
  }

  async get(sandboxId: string): Promise<Sandbox> {
    const result = await this.http.request<{ sandbox: Sandbox }>({
      method: "GET",
      path: `/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}`,
    });
    return result.sandbox;
  }

  async snapshot(
    sandboxId: string,
    options: SnapshotOptions = {},
  ): Promise<SnapshotResponse> {
    return this.http.request<SnapshotResponse>({
      method: "POST",
      path: `/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}/snapshot`,
      body: options,
    });
  }

  async setNetworkPolicy(
    sandboxId: string,
    policy: NetworkPolicy,
  ): Promise<NetworkPolicy> {
    return this.http.request<NetworkPolicy>({
      method: "POST",
      path: `/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}/network-policy`,
      body: policy,
    });
  }

  async createCredential(
    sandboxId: string,
    binding: CreateCredentialBindingRequest,
  ): Promise<CredentialBinding> {
    return this.http.request<CredentialBinding>({
      method: "POST",
      path: `/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}/credentials`,
      body: binding,
    });
  }

  async deleteCredential(
    sandboxId: string,
    credentialId: string,
  ): Promise<void> {
    await this.http.requestRaw({
      method: "DELETE",
      path: `/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}/credentials/${encodeURIComponent(credentialId)}`,
    });
  }

  async extend(sandboxId: string, additionalMs: number): Promise<Sandbox> {
    const body: ExtendTimeoutRequest = { additional_ms: additionalMs };
    return this.http.request<Sandbox>({
      method: "POST",
      path: `/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}/extend`,
      body,
    });
  }

  async stop(sandboxId: string, options: StopOptions = {}): Promise<void> {
    const query: Record<string, string> = {};
    if (options.force) query.force = "true";

    await this.http.requestRaw({
      method: "DELETE",
      path: `/pub/v1/sandboxes/${encodeURIComponent(sandboxId)}`,
      query,
    });
  }

  async getSnapshot(snapshotId: string): Promise<Snapshot> {
    return this.http.request<Snapshot>({
      method: "GET",
      path: `/pub/v1/snapshots/${encodeURIComponent(snapshotId)}`,
    });
  }

  async deleteSnapshot(snapshotId: string): Promise<void> {
    await this.http.requestRaw({
      method: "DELETE",
      path: `/pub/v1/snapshots/${encodeURIComponent(snapshotId)}`,
    });
  }

  async createSecret(request: CreateSecretRequest): Promise<CreateSecretResponse> {
    return this.http.request<CreateSecretResponse>({
      method: "POST",
      path: "/pub/v1/secrets",
      body: request,
    });
  }

  async deleteSecret(secretId: string): Promise<void> {
    await this.http.requestRaw({
      method: "DELETE",
      path: `/pub/v1/secrets/${encodeURIComponent(secretId)}`,
    });
  }
}
