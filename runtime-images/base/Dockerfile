FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    tini \
    ca-certificates \
    curl \
    iptables \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash sandbox

WORKDIR /workspace

COPY agentd /usr/local/bin/agentd

ENTRYPOINT ["tini", "--"]
CMD ["agentd"]

EXPOSE 8080
