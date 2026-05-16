FROM rust:1.80-slim AS builder

WORKDIR /build
COPY agentd/ ./
RUN cargo build --release && strip target/release/agentd-spike

FROM node:24-slim

COPY --from=builder /build/target/release/agentd-spike /usr/local/bin/agentd

ENV PORT=8080
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/agentd"]
