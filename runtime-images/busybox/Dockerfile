FROM debian:bookworm-slim AS libs

RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    tini \
    ca-certificates \
    jq \
    && rm -rf /var/lib/apt/lists/*

FROM busybox:1.37-glibc

COPY --from=libs /usr/bin/bash /usr/bin/bash
COPY --from=libs /usr/bin/jq /usr/bin/jq
COPY --from=libs /usr/bin/tini /usr/bin/tini
COPY --from=libs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

# bash, jq, and agentd runtime libs (glibc base in busybox:glibc)
COPY --from=libs /lib/x86_64-linux-gnu/libtinfo.so.6 /lib/x86_64-linux-gnu/
COPY --from=libs /lib/x86_64-linux-gnu/libonig.so.5 /lib/x86_64-linux-gnu/
COPY --from=libs /lib/x86_64-linux-gnu/libjq.so.1 /lib/x86_64-linux-gnu/
COPY --from=libs /lib/x86_64-linux-gnu/libm.so.6 /lib/x86_64-linux-gnu/
COPY --from=libs /lib/x86_64-linux-gnu/libgcc_s.so.1 /lib/x86_64-linux-gnu/

RUN ["busybox", "ln", "-sf", "/usr/bin/bash", "/bin/bash"]
RUN ["busybox", "ln", "-sf", "/usr/bin/bash", "/bin/sh"]
RUN ["busybox", "adduser", "-D", "-s", "/bin/bash", "-h", "/home/sandbox", "sandbox"]
RUN ["busybox", "mkdir", "-p", "/workspace"]

WORKDIR /workspace

COPY agentd /usr/local/bin/agentd

ENTRYPOINT ["tini", "--"]
CMD ["agentd"]

EXPOSE 8080
