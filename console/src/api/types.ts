export type BackendStatus =
  | "scheduled"
  | "loading"
  | "starting"
  | "waiting"
  | "ready"
  | "terminating"
  | "hard-terminating"
  | "terminated";

export type Sandbox = {
  id: string;
  /** Null after the Plane backend has been reaped past retention. */
  backend_id: string | null;
  tenant: string;
  status: BackendStatus;
  runtime: string;
  trust: "trusted" | "untrusted" | "paranoid";
  arch: "amd64" | "arm64";
  image: string;
  resources: {
    vcpus: number;
    memory_mb: number;
  };
  ports: number[];
  tags: Record<string, string>;
  persistent: boolean;
  snapshot_source?: string;
  created_at: string;
  updated_at: string;
  expires_at: string;
  ready_at?: string;
  terminated_at?: string;
  drone_name?: string;
  cluster: string;
};

export type ListSandboxesResponse = {
  items: Sandbox[];
  next_cursor?: string;
};

export type ApiError = {
  error: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
    request_id: string;
  };
};

export type ExecEntry = {
  id: number;
  command: string;
  stdout: string;
  stderr: string;
  exitCode: number;
  durationMs: number;
  timestamp: string;
};
