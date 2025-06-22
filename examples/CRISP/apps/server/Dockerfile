############### stage 0: base-dev ###############
ARG RUST_VERSION=1.85.0
ARG RISC0_VERSION=2.1.0
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
COPY crates/sdk/Cargo.toml crates/sdk/
COPY Cargo.toml Cargo.lock ./
COPY crates/compute-provider/Cargo.toml crates/compute-provider/

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

COPY Cargo.lock ./Cargo.lock
COPY Cargo.toml ./Cargo.toml

# find crates/* -name "Cargo.toml" -not -path "*/support/*" -printf "COPY %p %p\n"
COPY crates/aggregator/Cargo.toml crates/aggregator/Cargo.toml
COPY crates/bfv-helpers/Cargo.toml crates/bfv-helpers/Cargo.toml
COPY crates/cli/Cargo.toml crates/cli/Cargo.toml
COPY crates/compute-provider/Cargo.toml crates/compute-provider/Cargo.toml
COPY crates/config/Cargo.toml crates/config/Cargo.toml
COPY crates/crypto/Cargo.toml crates/crypto/Cargo.toml
COPY crates/data/Cargo.toml crates/data/Cargo.toml
COPY crates/entrypoint/Cargo.toml crates/entrypoint/Cargo.toml
COPY crates/events/Cargo.toml crates/events/Cargo.toml
COPY crates/evm/Cargo.toml crates/evm/Cargo.toml
COPY crates/evm-helpers/Cargo.toml crates/evm-helpers/Cargo.toml
COPY crates/fhe/Cargo.toml crates/fhe/Cargo.toml
COPY crates/fs/Cargo.toml crates/fs/Cargo.toml
COPY crates/indexer/Cargo.toml crates/indexer/Cargo.toml
COPY crates/init/Cargo.toml crates/init/Cargo.toml
COPY crates/keyshare/Cargo.toml crates/keyshare/Cargo.toml
COPY crates/logger/Cargo.toml crates/logger/Cargo.toml
COPY crates/net/Cargo.toml crates/net/Cargo.toml
COPY crates/request/Cargo.toml crates/request/Cargo.toml
COPY crates/sdk/Cargo.toml crates/sdk/Cargo.toml
COPY crates/sortition/Cargo.toml crates/sortition/Cargo.toml
COPY crates/support-scripts/Cargo.toml crates/support-scripts/Cargo.toml
COPY crates/test-helpers/Cargo.toml crates/test-helpers/Cargo.toml
COPY crates/tests/Cargo.toml crates/tests/Cargo.toml


RUN set -eux; \
    find \
    examples/CRISP/apps/program \
    crates \
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
COPY Cargo.*                        ./
COPY crates/           crates/
COPY crates/compute-provider/     crates/compute-provider/

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
