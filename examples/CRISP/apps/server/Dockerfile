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
COPY packages/enclave-sdk/Cargo.toml packages/enclave-sdk/
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
COPY packages/enclave-sdk/Cargo.toml packages/enclave-sdk/
COPY packages/compute_provider/Cargo.toml packages/compute_provider/

RUN set -eux; \
    find \
    examples/CRISP/apps/program \
    packages/enclave-sdk \
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
    --mount=type=cache,id=cargo-target,target=/app/examples/CRISP/apps/server/target,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json

############### stage 3: builder ###############
FROM base-dev AS builder

ARG SKIP_SOLIDITY
ENV SKIP_SOLIDITY=${SKIP_SOLIDITY}

WORKDIR /app

COPY examples/CRISP/apps/server/    examples/CRISP/apps/server/
COPY examples/CRISP/apps/program/   examples/CRISP/apps/program/
COPY packages/enclave-sdk/          packages/enclave-sdk/
COPY packages/compute_provider/     packages/compute_provider/

WORKDIR /app/examples/CRISP/apps/server
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=cargo-target,target=/app/examples/CRISP/apps/server/target,sharing=locked \
    cargo build --release --bin server && \
    mkdir -p /app/dist && \
    cp /app/examples/CRISP/apps/server/target/release/server /app/dist/server

############### stage 4: runtime ###############
FROM gcr.io/distroless/cc-debian12 AS runtime
WORKDIR /

COPY --from=builder /app/dist/server \
    /usr/local/bin/server
ENTRYPOINT ["/usr/local/bin/server"]
