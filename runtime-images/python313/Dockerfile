FROM python:3.13-slim-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    tini \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash sandbox || true

WORKDIR /workspace

COPY agentd /usr/local/bin/agentd

ENTRYPOINT ["tini", "--"]
CMD ["agentd"]

EXPOSE 8080
