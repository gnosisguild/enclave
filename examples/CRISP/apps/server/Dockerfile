############### stage 0: base-dev ###############
ARG RUST_VERSION=1.85.0
ARG RISC0_VERSION=2.0.0
ARG SKIP_SOLIDITY=0

FROM rust:${RUST_VERSION}-slim-bullseye AS base-dev

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    pkg-config libssl-dev curl ca-certificates && \
    rm -rf /var/lib/apt/lists/*

RUN curl -sSfL https://risczero.com/install | bash
ENV RISC0_HOME=/root/.risc0
ENV PATH="${RISC0_HOME}/bin:${PATH}"
RUN rzup install rust ${RUST_VERSION}
RUN rzup install r0vm ${RISC0_VERSION}
RUN rzup install cargo-risczero ${RISC0_VERSION}

RUN cargo install --locked cargo-chef

#################################################
############### stage 1: chef ###################
FROM base-dev AS chef
WORKDIR /app/examples/CRISP/apps/server

COPY examples/CRISP/apps/server/Cargo.toml   ./Cargo.toml
COPY examples/CRISP/apps/server/Cargo.lock   ./Cargo.lock
COPY examples/CRISP/apps/program/**/Cargo.*  /app/examples/CRISP/apps/program/
COPY packages/enclave-sdk/Cargo.*            /app/packages/enclave-sdk/
COPY packages/compute_provider/Cargo.*       /app/packages/compute_provider/

RUN cargo chef prepare --recipe-path recipe.json

############### stage 2: planner ###############
FROM base-dev AS planner

ARG SKIP_SOLIDITY
ENV SKIP_SOLIDITY=${SKIP_SOLIDITY}

WORKDIR /app
COPY --from=chef /app/examples/CRISP/apps/server/recipe.json /app/examples/CRISP/apps/server/recipe.json

COPY examples/CRISP/apps/server/    examples/CRISP/apps/server/
COPY examples/CRISP/apps/program/   examples/CRISP/apps/program/
COPY packages/enclave-sdk/          packages/enclave-sdk/
COPY packages/compute_provider/     packages/compute_provider/

WORKDIR /app/examples/CRISP/apps/server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
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

COPY --from=planner /app/examples/CRISP/apps/server/target /app/examples/CRISP/apps/server/target

WORKDIR /app/examples/CRISP/apps/server
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin server

############### stage 4: runtime ###############
FROM gcr.io/distroless/cc-debian12 AS runtime
WORKDIR /

COPY --from=builder /app/examples/CRISP/apps/server/target/release/server /usr/local/bin/server
ENTRYPOINT ["/usr/local/bin/server"]