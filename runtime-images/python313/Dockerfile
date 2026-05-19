FROM python:3.13-slim-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    tini \
    ca-certificates \
    curl \
    iptables \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash sandbox || true

WORKDIR /workspace

COPY agentd /usr/local/bin/agentd
COPY sandbox-entrypoint.sh /usr/local/bin/sandbox-entrypoint.sh
RUN chmod +x /usr/local/bin/sandbox-entrypoint.sh

ENTRYPOINT ["tini", "--", "sandbox-entrypoint.sh"]
CMD ["agentd"]

EXPOSE 8080
