FROM rust:1.88-bookworm AS builder

WORKDIR /work
COPY Cargo.toml /work/
COPY Cargo.lock /work/

# Stub out workspace members for dependency caching
COPY plane/Cargo.toml /work/plane/Cargo.toml
COPY plane-dynamic-proxy/Cargo.toml /work/plane-dynamic-proxy/Cargo.toml
COPY plane-common/Cargo.toml /work/plane-common/Cargo.toml
COPY plane-tests/Cargo.toml /work/plane-tests/Cargo.toml
COPY plane-test-macro/Cargo.toml /work/plane-test-macro/Cargo.toml
COPY common/Cargo.toml /work/common/Cargo.toml
COPY agentd/Cargo.toml /work/agentd/Cargo.toml
COPY egress-proxy/Cargo.toml /work/egress-proxy/Cargo.toml
COPY ops-cli/Cargo.toml /work/ops-cli/Cargo.toml
COPY tools/generate-schemas/Cargo.toml /work/tools/generate-schemas/Cargo.toml

RUN mkdir -p /work/plane/src && echo "fn main() {}" > /work/plane/src/main.rs
RUN mkdir -p /work/plane-dynamic-proxy/src && touch /work/plane-dynamic-proxy/src/lib.rs
RUN mkdir -p /work/plane-common/src && touch /work/plane-common/src/lib.rs
RUN mkdir -p /work/plane-tests/src && touch /work/plane-tests/src/lib.rs
RUN mkdir -p /work/plane-test-macro/src && touch /work/plane-test-macro/src/lib.rs
RUN mkdir -p /work/common/src && touch /work/common/src/lib.rs
RUN mkdir -p /work/agentd/src && echo "fn main() {}" > /work/agentd/src/main.rs
RUN mkdir -p /work/egress-proxy/src && echo "fn main() {}" > /work/egress-proxy/src/main.rs
RUN mkdir -p /work/ops-cli/src && echo "fn main() {}" > /work/ops-cli/src/main.rs
RUN mkdir -p /work/tools/generate-schemas/src && echo "fn main() {}" > /work/tools/generate-schemas/src/main.rs

# Pre-build dependencies
ENV SQLX_OFFLINE=true
RUN cargo build -p plane --release --locked 2>/dev/null || true

# Copy real source
COPY plane plane
COPY plane-dynamic-proxy plane-dynamic-proxy
COPY plane-common plane-common
COPY common common

# Build the plane binary
RUN cargo build -p plane --release --locked

FROM gcr.io/distroless/cc-debian12

COPY --from=builder /work/target/release/plane /bin/plane
ENTRYPOINT ["/bin/plane"]
