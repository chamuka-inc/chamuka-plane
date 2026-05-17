import { Sandbox } from "./sandbox.js";
import type { SandboxClient, SnapshotOptions, StopOptions } from "./client.js";
import type {
  Sandbox as SandboxType,
  SandboxConnection,
  SnapshotResponse,
  NetworkPolicy,
  CredentialBinding,
  CreateCredentialBindingRequest,
} from "./generated/types.js";

export class ManagedSandbox extends Sandbox {
  readonly id: string;
  readonly status: string;
  readonly runtime: string;
  private readonly client: SandboxClient;

  constructor(
    sandboxData: SandboxType,
    connection: SandboxConnection,
    client: SandboxClient,
  ) {
    const connectResponse = {
      backend_id: sandboxData.backend_id,
      spawned: true,
      status: sandboxData.status,
      token: connection.token,
      url: connection.url,
      status_url: connection.status_url,
      ...(connection.secret_token != null && { secret_token: connection.secret_token }),
    };
    super(connectResponse, connection.url);
    this.id = sandboxData.id;
    this.status = sandboxData.status;
    this.runtime = sandboxData.runtime;
    this.client = client;
  }

  async snapshot(options: SnapshotOptions = {}): Promise<SnapshotResponse> {
    return this.client.snapshot(this.id, options);
  }

  async setNetworkPolicy(policy: NetworkPolicy): Promise<NetworkPolicy> {
    return this.client.setNetworkPolicy(this.id, policy);
  }

  async createCredential(
    binding: CreateCredentialBindingRequest,
  ): Promise<CredentialBinding> {
    return this.client.createCredential(this.id, binding);
  }

  async deleteCredential(credentialId: string): Promise<void> {
    return this.client.deleteCredential(this.id, credentialId);
  }

  async extend(additionalMs: number): Promise<SandboxType> {
    return this.client.extend(this.id, additionalMs);
  }

  async stop(options: StopOptions = {}): Promise<void> {
    return this.client.stop(this.id, options);
  }
}
