FROM node:22 AS evm-builder

WORKDIR /build/packages/evm
COPY ./packages/evm ./

RUN npm install -g corepack
RUN corepack enable

RUN pnpm install && pnpm compile

# Build stage
FROM rust:1.81 AS ciphernode-builder

# Force incremental 
ENV CARGO_INCREMENTAL=1
ENV RUSTC_FORCE_INCREMENTAL=1
ENV CARGO_BUILD_JOBS=8

# Create build directory
WORKDIR /build/packages/ciphernode
COPY --from=evm-builder /build/packages/evm/artifacts ../evm/artifacts
COPY --from=evm-builder /build/packages/evm/deployments ../evm/deployments

# Copy workpace Cargo.toml
COPY ./packages/ciphernode/Cargo.toml ./Cargo.toml
COPY ./packages/ciphernode/Cargo.lock ./Cargo.lock
COPY ./packages/ciphernode/aggregator/Cargo.toml ./aggregator/Cargo.toml
COPY ./packages/ciphernode/config/Cargo.toml ./config/Cargo.toml
COPY ./packages/ciphernode/crypto/Cargo.toml ./crypto/Cargo.toml
COPY ./packages/ciphernode/data/Cargo.toml ./data/Cargo.toml
COPY ./packages/ciphernode/e3_request/Cargo.toml ./e3_request/Cargo.toml
COPY ./packages/ciphernode/enclave/Cargo.toml ./enclave/Cargo.toml
COPY ./packages/ciphernode/enclave_core/Cargo.toml ./enclave_core/Cargo.toml
COPY ./packages/ciphernode/enclave_core/build.rs ./enclave_core/build.rs
COPY ./packages/ciphernode/events/Cargo.toml ./events/Cargo.toml
COPY ./packages/ciphernode/evm/Cargo.toml ./evm/Cargo.toml
COPY ./packages/ciphernode/fhe/Cargo.toml ./fhe/Cargo.toml
COPY ./packages/ciphernode/keyshare/Cargo.toml ./keyshare/Cargo.toml
COPY ./packages/ciphernode/logger/Cargo.toml ./logger/Cargo.toml
COPY ./packages/ciphernode/net/Cargo.toml ./net/Cargo.toml
COPY ./packages/ciphernode/sortition/Cargo.toml ./sortition/Cargo.toml
COPY ./packages/ciphernode/test_helpers/Cargo.toml ./test_helpers/Cargo.toml
COPY ./packages/ciphernode/tests/Cargo.toml ./tests/Cargo.toml
COPY ./packages/ciphernode/rust-toolchain.toml .

# Copy enclave-sdk package
COPY ./packages/enclave-sdk ../enclave-sdk
COPY ./packages/enclave-sdk-isomorphic ../enclave-sdk-isomorphic

RUN for d in ./*/ ; do \
        if [ -f "$d/Cargo.toml" ]; then \
            mkdir -p "$d/src" && \
            echo "fn main() {}" > "$d/src/lib.rs"; \
        fi \
    done

RUN cargo build --release

COPY ./packages/ciphernode .

RUN find . -name "*.rs" -exec touch {} +
RUN cargo build --release

# Runtime stage
FROM debian:stable-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends iptables dnsutils iputils-ping ca-certificates jq && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash ciphernode

# Create necessary directories with proper permissions
RUN mkdir -p /home/ciphernode/.config/enclave \
    /home/ciphernode/.local/share/enclave \
    /run/secrets && \
    chown -R ciphernode:ciphernode /home/ciphernode /run/secrets

# Switch to non-root user
USER ciphernode
WORKDIR /home/ciphernode

# Copy binary from builder
COPY --from=ciphernode-builder --chown=ciphernode:ciphernode /build/packages/ciphernode/target/release/enclave /usr/local/bin/
COPY --from=ciphernode-builder --chmod=755 --chown=ciphernode:ciphernode /build/packages/ciphernode/ciphernode-entrypoint.sh /usr/local/bin/

# Environment variables for configuration
ENV CONFIG_DIR=/home/ciphernode/.config/enclave
ENV DATA_DIR=/home/ciphernode/.local/share/enclave

# Add entrypoint script
ENTRYPOINT ["ciphernode-entrypoint.sh"]
