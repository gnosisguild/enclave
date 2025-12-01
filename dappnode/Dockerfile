# DAppNode Enclave Ciphernode Dockerfile
# Uses official upstream image from GitHub Container Registry
# For local testing: use Dockerfile.local instead

FROM hmzakhalid/enclave-ciphernode:0.1.0 AS upstream

# Runtime image
FROM debian:stable-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    procps \
    gettext-base \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash ciphernode

# Create data directory
RUN mkdir -p /data && chown ciphernode:ciphernode /data

# Copy binary from upstream
COPY --from=upstream /usr/local/bin/enclave /usr/local/bin/enclave
RUN chmod +x /usr/local/bin/enclave

# Copy entrypoint and config template
COPY --chmod=755 entrypoint.sh /usr/local/bin/entrypoint.sh
COPY --chmod=644 config.template.yaml /opt/config.template.yaml

# Labels
LABEL maintainer="Gnosis Guild <https://gnosisguild.org>"
LABEL org.opencontainers.image.source="https://github.com/gnosisguild/enclave"
LABEL org.opencontainers.image.description="Enclave Ciphernode for DAppNode"

USER ciphernode
WORKDIR /data

HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD pgrep -f enclave || exit 1

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
