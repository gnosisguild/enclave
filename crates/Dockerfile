FROM node:22 AS evm-builder

WORKDIR /build
COPY crates/wasm/package.json ./crates/wasm/package.json
COPY packages/enclave-react ./packages/enclave-react
COPY packages/enclave-sdk ./packages/enclave-sdk
COPY packages/enclave-config ./packages/enclave-config
COPY packages/enclave-contracts ./packages/enclave-contracts
COPY pnpm-workspace.yaml .
COPY pnpm-lock.yaml .

RUN npm install -g corepack
RUN corepack enable

# We just need the contracts to be compiled here
RUN cd packages/enclave-contracts && pnpm install && pnpm compile:contracts

# Build stage
FROM rust:1.86 AS ciphernode-builder

# Force incremental 
ENV CARGO_INCREMENTAL=1
ENV RUSTC_FORCE_INCREMENTAL=1
ENV CARGO_BUILD_JOBS=8

# Install deps
RUN apt-get update && apt-get install -y --no-install-recommends jq
RUN wget https://github.com/argotorg/solidity/releases/download/v0.8.27/solc-static-linux \
    && chmod +x solc-static-linux \
    && mv solc-static-linux /usr/local/bin/solc \
    && solc --version

# Create build directory

WORKDIR /build
COPY Cargo.lock ./Cargo.lock
COPY Cargo.toml ./Cargo.toml
COPY ./rust-toolchain.toml .

WORKDIR /build/crates
COPY --from=evm-builder /build/packages/enclave-contracts/artifacts ../packages/enclave-contracts/artifacts
COPY --from=evm-builder /build/packages/enclave-contracts/deployed_contracts.json ../packages/enclave-contracts/deployed_contracts.json

# find crates/* -name "Cargo.toml" -not -path "*/support/*" -printf "COPY %p %p\n" | sed 's|COPY \(.*\) crates/|COPY \1 ./|'
COPY crates/aggregator/Cargo.toml ./aggregator/Cargo.toml
COPY crates/bfv-helpers/Cargo.toml ./bfv-helpers/Cargo.toml
COPY crates/cli/Cargo.toml ./cli/Cargo.toml
COPY crates/ciphernode-builder/Cargo.toml ./ciphernode-builder/Cargo.toml
COPY crates/compute-provider/Cargo.toml ./compute-provider/Cargo.toml
COPY crates/config/Cargo.toml ./config/Cargo.toml
COPY crates/crypto/Cargo.toml ./crypto/Cargo.toml
COPY crates/data/Cargo.toml ./data/Cargo.toml
COPY crates/enclaveup/Cargo.toml ./enclaveup/Cargo.toml
COPY crates/entrypoint/Cargo.toml ./entrypoint/Cargo.toml
COPY crates/events/Cargo.toml ./events/Cargo.toml
COPY crates/evm/Cargo.toml ./evm/Cargo.toml
COPY crates/evm-helpers/Cargo.toml ./evm-helpers/Cargo.toml
COPY crates/fhe/Cargo.toml ./fhe/Cargo.toml
COPY crates/fs/Cargo.toml ./fs/Cargo.toml
COPY crates/hamt/Cargo.toml ./hamt/Cargo.toml
COPY crates/indexer/Cargo.toml ./indexer/Cargo.toml
COPY crates/init/Cargo.toml ./init/Cargo.toml
COPY crates/keyshare/Cargo.toml ./keyshare/Cargo.toml
COPY crates/logger/Cargo.toml ./logger/Cargo.toml
COPY crates/multithread/Cargo.toml ./multithread/Cargo.toml
COPY crates/net/Cargo.toml ./net/Cargo.toml
COPY crates/program-server/Cargo.toml ./program-server/Cargo.toml
COPY crates/request/Cargo.toml ./request/Cargo.toml
COPY crates/sdk/Cargo.toml ./sdk/Cargo.toml
COPY crates/sortition/Cargo.toml ./sortition/Cargo.toml
COPY crates/support-scripts/Cargo.toml ./support-scripts/Cargo.toml
COPY crates/test-helpers/Cargo.toml ./test-helpers/Cargo.toml
COPY crates/tests/Cargo.toml ./tests/Cargo.toml
COPY crates/trbfv/Cargo.toml ./trbfv/Cargo.toml
COPY crates/utils/Cargo.toml ./utils/Cargo.toml
COPY crates/wasm/Cargo.toml ./wasm/Cargo.toml

RUN echo 'fn main() { println!("cargo:warning=dependency cache build"); }' > ./entrypoint/build.rs
RUN echo 'fn main() { println!("cargo:warning=dependency cache build"); }' > ./cli/build.rs
RUN for d in ./*/ ; do \
        if [ -f "$d/Cargo.toml" ]; then \
          if [ "$d" = "./cli/" ] || [ "$d" = "./enclaveup/" ]; then \
            mkdir -p "$d/src" && \
            echo "fn main() {}" > "$d/src/main.rs"; \
          else \
            mkdir -p "$d/src" && \
            echo "pub fn main() {}" > "$d/src/lib.rs"; \
          fi \
        fi \
    done

RUN cargo build --locked --release

COPY ./crates .

RUN find . -name "*.rs" -exec touch {} +
RUN cargo build --locked --release -p e3-cli

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
COPY --from=ciphernode-builder --chown=ciphernode:ciphernode /build/target/release/enclave /usr/local/bin/
COPY --from=ciphernode-builder --chmod=755 --chown=ciphernode:ciphernode /build/crates/ciphernode-entrypoint.sh /usr/local/bin/

# Environment variables for configuration
ENV CONFIG_DIR=/home/ciphernode/.config/enclave
ENV DATA_DIR=/home/ciphernode/.local/share/enclave

# Add entrypoint script
ENTRYPOINT ["ciphernode-entrypoint.sh"]
