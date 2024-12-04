FROM node:22 AS evm-builder

WORKDIR /build/packages/evm
COPY ./packages/evm ./
RUN yarn install && yarn compile

# Build stage
FROM rust:1.81 AS ciphernode-builder

# Create build directory
WORKDIR /build/packages/ciphernode
COPY ./packages/ciphernode ./
COPY --from=evm-builder /build/packages/evm/artifacts ../evm/artifacts
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends iptables ca-certificates jq && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash ciphernode

# Create necessary directories with proper permissions
RUN mkdir -p /home/ciphernode/.config/enclave \
    /home/ciphernode/.local/share/enclave \
    /home/ciphernode/secrets && \
    chown -R ciphernode:ciphernode /home/ciphernode && \
    chmod 700 /home/ciphernode/secrets

# Switch to non-root user
USER ciphernode
WORKDIR /home/ciphernode

# Copy binary from builder
COPY --from=ciphernode-builder --chown=ciphernode:ciphernode /build/packages/ciphernode/target/release/enclave /usr/local/bin/

# Environment variables for configuration
ENV CONFIG_DIR=/home/ciphernode/.config/enclave
ENV SECRETS_DIR=/home/ciphernode/secrets
ENV DATA_DIR=/home/ciphernode/.local/share/enclave
ENV RUST_LOG=info

# Add entrypoint script
COPY --from=ciphernode-builder --chmod=755 --chown=ciphernode:ciphernode /build/packages/ciphernode/ciphernode-entrypoint.sh /usr/local/bin/

ENTRYPOINT ["ciphernode-entrypoint.sh"]