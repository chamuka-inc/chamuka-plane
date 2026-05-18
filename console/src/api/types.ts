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
  backend_id: string;
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

export type UsageRecord = {
  sandbox_id: string;
  bucket_start: string;
  cpu_nsec: number;
  memory_mb_seconds: number;
  rx_bytes: number;
  tx_bytes: number;
};

export type UsageSummary = {
  total_cpu_hours: number;
  total_memory_gb_hours: number;
  total_rx_bytes: number;
  total_tx_bytes: number;
  estimated_cost: number;
};

export type UsageResponse = {
  records: UsageRecord[];
  summary: UsageSummary;
};

export type DroneState = {
  ready: boolean;
  draining: boolean;
  last_heartbeat_age: number;
  backend_count: number;
  node: NodeState;
};

export type NodeState = {
  name: string;
  plane_version: string;
  plane_hash: string;
  controller: string;
  controller_heartbeat_age: number;
};

export type ClusterState = {
  name: string;
  drones: DroneState[];
  proxies: NodeState[];
};

export type FleetResponse = {
  clusters: ClusterState[];
};

export type ExtensionAction = {
  id: string;
  kind: "push_policy" | "push_credential" | "revoke_credential" | "snapshot";
  backend_id?: string;
  sandbox_id?: string;
  drone_id: number;
  created_at: string;
  acked_at?: string;
};

export type StatusEvent = {
  status: BackendStatus;
  termination_reason?: string;
  termination_kind?: "soft" | "hard";
  exit_error?: boolean;
  time: string;
};

export type ApiError = {
  error: {
    code: string;
    message: string;
    details?: Record<string, unknown>;
    request_id: string;
  };
};
