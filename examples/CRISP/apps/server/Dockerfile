############### stage 0: base-dev ###############
ARG RUST_VERSION=1.85.0
ARG RISC0_VERSION=2.0.0
ARG SKIP_SOLIDITY=0

FROM rust:${RUST_VERSION}-slim-bullseye AS base-dev

ENV RISC0_HOME=/root/.risc0
ENV PATH="${RISC0_HOME}/bin:${PATH}"
ENV CARGO_HOME=/usr/local/cargo

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config libssl-dev curl ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN curl -sSfL https://risczero.com/install | bash
RUN rzup install rust ${RUST_VERSION}
RUN rzup install r0vm ${RISC0_VERSION}
RUN rzup install cargo-risczero ${RISC0_VERSION}

RUN cargo install --locked cargo-chef

#################################################
############### stage 1: chef ###################
FROM base-dev AS chef
WORKDIR /app

COPY examples/CRISP/apps/server/Cargo.toml examples/CRISP/apps/server/Cargo.lock examples/CRISP/apps/server/
COPY examples/CRISP/apps/program/Cargo.toml examples/CRISP/apps/program/
COPY examples/CRISP/apps/program/host/Cargo.toml examples/CRISP/apps/program/host/
COPY examples/CRISP/apps/program/core/Cargo.toml examples/CRISP/apps/program/core/
COPY examples/CRISP/apps/program/methods/Cargo.toml examples/CRISP/apps/program/methods/
COPY packages/ciphernode/sdk/Cargo.toml packages/ciphernode/sdk/
COPY packages/ciphernode/Cargo.toml packages/ciphernode/Cargo.lock packages/ciphernode/
COPY packages/compute_provider/Cargo.toml packages/compute_provider/

WORKDIR /app/examples/CRISP/apps/server
RUN cargo chef prepare --recipe-path recipe.json

############### stage 2: planner ###############
FROM base-dev AS planner

ARG SKIP_SOLIDITY
ENV SKIP_SOLIDITY=${SKIP_SOLIDITY}

WORKDIR /app
COPY --from=chef /app/examples/CRISP/apps/server/recipe.json \
    /app/examples/CRISP/apps/server/recipe.json

COPY examples/CRISP/apps/server/Cargo.toml examples/CRISP/apps/server/Cargo.lock examples/CRISP/apps/server/
COPY examples/CRISP/apps/program/Cargo.toml examples/CRISP/apps/program/
COPY examples/CRISP/apps/program/host/Cargo.toml examples/CRISP/apps/program/host/
COPY examples/CRISP/apps/program/core/Cargo.toml examples/CRISP/apps/program/core/
COPY examples/CRISP/apps/program/methods/Cargo.toml examples/CRISP/apps/program/methods/

# find packages/ciphernode/* -name "Cargo.*" -printf "COPY %p %p\n"
COPY packages/ciphernode/aggregator/Cargo.toml packages/ciphernode/aggregator/Cargo.toml
COPY packages/ciphernode/bfv-helpers/Cargo.toml packages/ciphernode/bfv-helpers/Cargo.toml
COPY packages/ciphernode/Cargo.lock packages/ciphernode/Cargo.lock
COPY packages/ciphernode/Cargo.toml packages/ciphernode/Cargo.toml
COPY packages/ciphernode/config/Cargo.toml packages/ciphernode/config/Cargo.toml
COPY packages/ciphernode/crypto/Cargo.toml packages/ciphernode/crypto/Cargo.toml
COPY packages/ciphernode/data/Cargo.toml packages/ciphernode/data/Cargo.toml
COPY packages/ciphernode/e3_request/Cargo.toml packages/ciphernode/e3_request/Cargo.toml
COPY packages/ciphernode/enclave/Cargo.toml packages/ciphernode/enclave/Cargo.toml
COPY packages/ciphernode/enclave_core/Cargo.toml packages/ciphernode/enclave_core/Cargo.toml
COPY packages/ciphernode/events/Cargo.toml packages/ciphernode/events/Cargo.toml
COPY packages/ciphernode/evm/Cargo.toml packages/ciphernode/evm/Cargo.toml
COPY packages/ciphernode/evm-helpers/Cargo.toml packages/ciphernode/evm-helpers/Cargo.toml
COPY packages/ciphernode/fhe/Cargo.toml packages/ciphernode/fhe/Cargo.toml
COPY packages/ciphernode/indexer/Cargo.toml packages/ciphernode/indexer/Cargo.toml
COPY packages/ciphernode/init/Cargo.toml packages/ciphernode/init/Cargo.toml
COPY packages/ciphernode/keyshare/Cargo.toml packages/ciphernode/keyshare/Cargo.toml
COPY packages/ciphernode/logger/Cargo.toml packages/ciphernode/logger/Cargo.toml
COPY packages/ciphernode/net/Cargo.toml packages/ciphernode/net/Cargo.toml
COPY packages/ciphernode/sdk/Cargo.toml packages/ciphernode/sdk/Cargo.toml
COPY packages/ciphernode/sortition/Cargo.toml packages/ciphernode/sortition/Cargo.toml
COPY packages/ciphernode/test_helpers/Cargo.toml packages/ciphernode/test_helpers/Cargo.toml
COPY packages/ciphernode/tests/Cargo.toml packages/ciphernode/tests/Cargo.toml

COPY packages/compute_provider/Cargo.toml packages/compute_provider/

RUN set -eux; \
    find \
    examples/CRISP/apps/program \
    packages/ciphernode \
    packages/compute_provider \
    -type f -name Cargo.toml ! -path '*/target/*' -print0 \
    | while IFS= read -r -d '' manifest_path; do \
        crate_dir="$(dirname "$manifest_path")"; \
        if [ "$manifest_path" = "examples/CRISP/apps/program/Cargo.toml" ]; then \
            continue; \
        fi; \
        mkdir -p "$crate_dir/src"; \
        if [ ! -f "$crate_dir/src/lib.rs" ] && [ ! -f "$crate_dir/src/main.rs" ]; then \
            echo "// Stub lib.rs for $crate_dir" > "$crate_dir/src/lib.rs"; \
        fi; \
    done

WORKDIR /app/examples/CRISP/apps/server
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cargo-target,target=/app/target,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

############### stage 3: builder ###############
FROM base-dev AS builder

ARG SKIP_SOLIDITY
ENV SKIP_SOLIDITY=${SKIP_SOLIDITY}

WORKDIR /app

COPY examples/CRISP/apps/server/    examples/CRISP/apps/server/
COPY examples/CRISP/apps/program/   examples/CRISP/apps/program/
COPY packages/ciphernode/           packages/ciphernode/
COPY packages/compute_provider/     packages/compute_provider/

WORKDIR /app/examples/CRISP/apps/server
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cargo-target,target=/app/target,sharing=locked \
    cargo build --release --bin server && \
    mkdir -p /app/dist && cp target/release/server /app/dist/server

############### stage 4: runtime ###############
FROM gcr.io/distroless/cc-debian12 AS runtime
WORKDIR /

COPY --from=builder /app/dist/server \
    /usr/local/bin/server
ENTRYPOINT ["/usr/local/bin/server"]
