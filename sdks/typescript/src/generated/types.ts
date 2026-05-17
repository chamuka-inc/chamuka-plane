export type Arch = "amd64" | "arm64";

export type TrustLevel = "trusted" | "untrusted" | "paranoid";

export type BackendStatus =
  | "scheduled"
  | "loading"
  | "starting"
  | "waiting"
  | "ready"
  | "terminating"
  | "hard-terminating"
  | "terminated";

export type SandboxResources = {
  vcpus: number;
  memory_mb?: number;
};

export type NetworkPolicyMode = "allow-all" | "deny-all" | "rules";

export type RuleAction = "allow" | "deny";

export type NetworkRule = {
  domains?: string[] | null;
  cidrs?: string[] | null;
  ports?: number[] | null;
  action?: RuleAction | null;
  inject?: string | null;
};

export type NetworkPolicy = {
  mode: NetworkPolicyMode;
  rules?: NetworkRule[] | null;
};

export type SandboxSource =
  | { type: "git"; repo: string; branch?: string | null; commit?: string | null }
  | { type: "snapshot"; snapshot_id: string }
  | { type: "image"; image: string };

export type CreateSandboxRequest = {
  runtime: string;
  image?: string | null;
  trust?: TrustLevel;
  arch?: Arch;
  resources: SandboxResources;
  timeout_ms?: number | null;
  source?: SandboxSource | null;
  ports?: number[];
  env?: Record<string, string>;
  tags?: Record<string, string>;
  persistent?: boolean;
  initial_network_policy?: NetworkPolicy | null;
  use_static_token?: boolean;
  key?: string | null;
};

export type Sandbox = {
  id: string;
  backend_id: string;
  tenant: string;
  status: BackendStatus;
  runtime: string;
  trust: TrustLevel;
  arch: Arch;
  image: string;
  resources: SandboxResources;
  ports: number[];
  tags: Record<string, string>;
  persistent: boolean;
  snapshot_source?: string | null;
  created_at: string;
  updated_at: string;
  expires_at: string;
  ready_at?: string | null;
  terminated_at?: string | null;
  drone_name?: string | null;
  cluster: string;
};

export type SandboxConnection = {
  token: string;
  url: string;
  secret_token?: string | null;
  status_url: string;
};

export type CreateSandboxResponse = {
  sandbox: Sandbox;
  connection: SandboxConnection;
};

export type ListSandboxesResponse = {
  items: Sandbox[];
  next_cursor?: string | null;
};

export type ExecRequest = {
  cmd: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string | null;
  stdin?: string | null;
  timeout_ms?: number | null;
  detached?: boolean;
  pty?: boolean;
  pty_size?: PtySize | null;
  user?: string | null;
};

export type PtySize = {
  rows: number;
  cols: number;
};

export type ExecSyncResponse = {
  cmd_id: string;
  exit_code: number;
  stdout: string;
  stderr: string;
  duration_ms: number;
  truncated?: boolean;
};

export type ExecDetachedResponse = {
  cmd_id: string;
  status: "running";
};

export type LogErrorCode = "BACKPRESSURE" | "TIMEOUT";

export type LogFrame =
  | { stream: "stdout"; data: string }
  | { stream: "stderr"; data: string }
  | { stream: "exit"; code: number; signal?: number | null }
  | { stream: "error"; code: LogErrorCode };

export type SignalName = "TERM" | "KILL" | "INT" | "HUP" | "USR1" | "USR2";

export type SignalRequest = {
  signal: SignalName;
};

export type MkdirRequest = {
  path: string;
  mode?: string | null;
  recursive?: boolean;
};

export type EntryType = "file" | "dir" | "symlink" | "other";

export type DirEntry = {
  name: string;
  type: EntryType;
  size?: number | null;
  mode: number;
  modified_at: string;
};

export type ListResponse = {
  path: string;
  entries: DirEntry[];
};

export type FsEventKind = "create" | "modify" | "delete" | "rename" | "attrib";

export type FsEvent = {
  event: FsEventKind;
  path: string;
  is_dir: boolean;
  at: string;
};

export type PortProtocol = "tcp" | "udp";

export type ListeningPort = {
  port: number;
  protocol: PortProtocol;
  process_pid?: number | null;
  process_name?: string | null;
};

export type PortsResponse = {
  listening: ListeningPort[];
};

export type MoveRequest = {
  source: string;
  destination: string;
};

export type ExecStatusResponse = {
  cmd_id: string;
  status: "running" | "exited";
  exit_code?: number | null;
};

export type StdinRequest = {
  data: string;
};

export type ConnectRequest = {
  key?: KeyConfig;
  spawn_config?: SpawnConfig;
  user?: string;
  auth?: Record<string, unknown>;
};

export type KeyConfig = {
  name: string;
  namespace?: string;
  tag?: string;
};

export type SpawnConfig = {
  id?: string;
  cluster?: string;
  pool?: string;
  executable: Record<string, unknown>;
  lifetime_limit_seconds?: number;
  max_idle_seconds?: number;
  use_static_token?: boolean;
};

export type ConnectResponse = {
  backend_id: string;
  spawned: boolean;
  status: BackendStatus;
  token: string;
  url: string;
  subdomain?: string;
  secret_token?: string;
  status_url: string;
  drone?: string;
};

export type SnapshotRequest = {
  sandbox_id: string;
  tag?: string | null;
};

export type Snapshot = {
  id: string;
  sandbox_id: string;
  tenant?: string;
  image_ref?: string;
  image?: string;
  digest?: string;
  base_image?: string;
  size_bytes?: number | null;
  arch?: Arch;
  runtime: string;
  name?: string | null;
  tag?: string | null;
  tags?: Record<string, string>;
  created_at: string;
  expires_at?: string | null;
};

export type SnapshotResponse = {
  snapshot: Snapshot;
};

export type ExtendTimeoutRequest = {
  additional_ms: number;
};

export type CreateSecretRequest = {
  name: string;
  value: string;
  description?: string | null;
};

export type CreateSecretResponse = {
  id: string;
  name: string;
  created_at: string;
};

export type CreateCredentialBindingRequest = {
  secret_id: string;
  env_var: string;
};

export type CredentialBinding = {
  id: string;
  secret_id: string;
  env_var: string;
  created_at: string;
};
