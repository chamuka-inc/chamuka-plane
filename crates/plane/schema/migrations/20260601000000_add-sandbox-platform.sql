-- Sandbox Platform schema extensions.
-- Adds our tables that reference Plane's existing backend/drone/node tables.
-- Forward-only; no DROP statements.

-- Tenants (lightweight; populated by the front-door auth integration).
CREATE TABLE tenant (
    id          VARCHAR(255) PRIMARY KEY,
    name        TEXT,
    created_at  TIMESTAMPTZ DEFAULT now() NOT NULL,
    deleted_at  TIMESTAMPTZ
);

-- Our wrapper over Plane's backend table. One row per platform sandbox.
CREATE TABLE sandbox (
    id              VARCHAR(255) PRIMARY KEY,
    backend_id      VARCHAR(255) NOT NULL UNIQUE REFERENCES backend(id),
    tenant          VARCHAR(255) NOT NULL REFERENCES tenant(id),

    runtime         VARCHAR(255) NOT NULL,
    trust_tier      VARCHAR(64)  NOT NULL,
    arch            VARCHAR(16)  NOT NULL,
    image           TEXT NOT NULL,

    vcpus           INTEGER NOT NULL,
    memory_mb       INTEGER NOT NULL,

    ports           INTEGER[] NOT NULL DEFAULT '{}',
    tags            JSONB     NOT NULL DEFAULT '{}',

    persistent      BOOLEAN   NOT NULL DEFAULT false,
    snapshot_source VARCHAR(255),
    network_policy  JSONB,

    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    ready_at        TIMESTAMPTZ,
    terminated_at   TIMESTAMPTZ
);
CREATE INDEX idx_sandbox_tenant ON sandbox (tenant, created_at DESC);
CREATE INDEX idx_sandbox_tags ON sandbox USING gin (tags);
CREATE INDEX idx_sandbox_backend ON sandbox (backend_id);

-- Snapshots.
CREATE TABLE snapshot (
    id            VARCHAR(255) PRIMARY KEY,
    sandbox_id    VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    tenant        VARCHAR(255) NOT NULL REFERENCES tenant(id),

    image_ref     TEXT NOT NULL,
    digest        VARCHAR(128) NOT NULL,
    base_image    TEXT NOT NULL,
    size_bytes    BIGINT NOT NULL,
    arch          VARCHAR(16) NOT NULL,
    runtime       VARCHAR(255) NOT NULL,

    name          TEXT,
    tags          JSONB NOT NULL DEFAULT '{}',

    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at    TIMESTAMPTZ NOT NULL
);
CREATE INDEX idx_snapshot_tenant ON snapshot (tenant, created_at DESC);
CREATE INDEX idx_snapshot_sandbox ON snapshot (sandbox_id);
CREATE INDEX idx_snapshot_expires_at ON snapshot (expires_at)
    WHERE expires_at IS NOT NULL;

-- Now add the FK from sandbox.snapshot_source → snapshot(id).
ALTER TABLE sandbox
    ADD CONSTRAINT fk_sandbox_snapshot_source
    FOREIGN KEY (snapshot_source) REFERENCES snapshot(id);

-- Network policies (one row per sandbox).
CREATE TABLE network_policy (
    sandbox_id VARCHAR(255) PRIMARY KEY REFERENCES sandbox(id) ON DELETE CASCADE,
    version    BIGINT NOT NULL DEFAULT 1,
    mode       VARCHAR(64) NOT NULL,
    rules      JSONB NOT NULL DEFAULT '[]',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Secret material (KMS-encrypted at rest).
CREATE TABLE secret (
    id                VARCHAR(255) PRIMARY KEY,
    tenant            VARCHAR(255) NOT NULL REFERENCES tenant(id),
    name              TEXT,
    kms_key_id        TEXT NOT NULL,
    wrapped_data_key  BYTEA NOT NULL,
    nonce             BYTEA NOT NULL,
    ciphertext        BYTEA NOT NULL,
    auth_tag          BYTEA NOT NULL,
    tags              JSONB NOT NULL DEFAULT '{}',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at        TIMESTAMPTZ
);
CREATE INDEX idx_secret_tenant ON secret (tenant) WHERE deleted_at IS NULL;

-- Credential bindings (link a secret to a sandbox + destination + header).
CREATE TABLE credential_binding (
    id            VARCHAR(255) PRIMARY KEY,
    sandbox_id    VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    secret_id     VARCHAR(255) NOT NULL REFERENCES secret(id),
    destination   TEXT NOT NULL,
    header_name   TEXT NOT NULL,
    header_format TEXT NOT NULL DEFAULT '{value}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at    TIMESTAMPTZ
);
CREATE INDEX idx_credential_sandbox ON credential_binding (sandbox_id)
    WHERE revoked_at IS NULL;
CREATE INDEX idx_credential_secret ON credential_binding (secret_id);

-- Outbound action queue for our extensions (policy/credential push, snapshot).
CREATE TABLE extension_action (
    id         VARCHAR(255) PRIMARY KEY,
    kind       VARCHAR(64) NOT NULL,
    backend_id VARCHAR(255) REFERENCES backend(id),
    sandbox_id VARCHAR(255) REFERENCES sandbox(id),
    drone_id   INTEGER NOT NULL REFERENCES drone(id),
    payload    JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    acked_at   TIMESTAMPTZ
);
CREATE INDEX idx_extension_action_pending ON extension_action (drone_id, created_at)
    WHERE acked_at IS NULL;

-- Usage metering (hourly buckets, one row per sandbox per hour).
CREATE TABLE usage_record (
    sandbox_id        VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    bucket_start      TIMESTAMPTZ NOT NULL,
    cpu_nsec          BIGINT NOT NULL,
    memory_mb_seconds BIGINT NOT NULL,
    rx_bytes          BIGINT NOT NULL DEFAULT 0,
    tx_bytes          BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (sandbox_id, bucket_start)
);
CREATE INDEX idx_usage_record_bucket ON usage_record (bucket_start);

-- Subdomain-port routing entries (one row per (sandbox, exposed port) pair).
CREATE TABLE port_route (
    sandbox_id      VARCHAR(255) NOT NULL REFERENCES sandbox(id) ON DELETE CASCADE,
    port            INTEGER NOT NULL,
    subdomain_token VARCHAR(255) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (sandbox_id, port)
);
CREATE UNIQUE INDEX idx_port_route_subdomain ON port_route (subdomain_token);

-- Audit log: egress proxy decisions, credential injections.
CREATE TABLE audit_egress (
    id                SERIAL PRIMARY KEY,
    at                TIMESTAMPTZ NOT NULL,
    sandbox_id        VARCHAR(255) NOT NULL REFERENCES sandbox(id),
    destination_host  TEXT NOT NULL,
    destination_port  INTEGER NOT NULL,
    destination_ip    INET,
    protocol          VARCHAR(16) NOT NULL,
    action            VARCHAR(32) NOT NULL,
    policy_rule_index INTEGER,
    bytes_sent        BIGINT NOT NULL DEFAULT 0,
    bytes_received    BIGINT NOT NULL DEFAULT 0,
    duration_ms       INTEGER NOT NULL DEFAULT 0,
    tls_sni           TEXT,
    http_host         TEXT,
    http_path         TEXT,
    http_status       INTEGER
);
CREATE INDEX idx_audit_egress_sandbox ON audit_egress (sandbox_id, at DESC);
CREATE INDEX idx_audit_egress_at ON audit_egress (at);
