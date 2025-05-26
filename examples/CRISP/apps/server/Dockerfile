############### stage 0: base-dev ###############
ARG RUST_VERSION=1.85.0

FROM rust:${RUST_VERSION}-slim-bullseye AS base

ENV CARGO_HOME=/usr/local/cargo \
    CARGO_TARGET_DIR=/app/target

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates; \
    rm -rf /var/lib/apt/lists/*


#################################################
############### stage 1: deps ###################
FROM base AS deps
WORKDIR /app

COPY examples/CRISP/apps/server/Cargo.toml examples/CRISP/apps/server/Cargo.lock examples/CRISP/apps/server/
COPY examples/CRISP/apps/program/Cargo.toml examples/CRISP/apps/program/
COPY examples/CRISP/apps/program/client/Cargo.toml examples/CRISP/apps/program/client/
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

RUN --mount=type=cache,id=cargo-registry,target=${CARGO_HOME}/registry,sharing=locked \
    --mount=type=cache,id=cargo-git,target=${CARGO_HOME}/git,sharing=locked       \
    --mount=type=cache,id=cargo-target,target=${CARGO_TARGET_DIR},sharing=locked \
    cargo build --release --manifest-path examples/CRISP/apps/server/Cargo.toml || true

#################################################
############### stage 2: builder ###############
FROM base AS builder
WORKDIR /app

COPY examples/CRISP/apps/server/    examples/CRISP/apps/server/
COPY examples/CRISP/apps/program/   examples/CRISP/apps/program/
COPY packages/enclave-sdk/          packages/enclave-sdk/
COPY packages/compute_provider/     packages/compute_provider/

RUN --mount=type=cache,id=cargo-registry,target=${CARGO_HOME}/registry,sharing=locked \
    --mount=type=cache,id=cargo-git,target=${CARGO_HOME}/git,sharing=locked \
    --mount=type=cache,id=cargo-target,target=${CARGO_TARGET_DIR},sharing=locked \
    cargo build --release --manifest-path examples/CRISP/apps/server/Cargo.toml --bin server && \
    mkdir -p /app/dist && cp ${CARGO_TARGET_DIR}/release/server /app/dist/server

############### stage 3: runtime ###############
FROM gcr.io/distroless/cc-debian12 AS runtime
COPY --from=builder /app/dist/server /usr/local/bin/server
ENTRYPOINT ["/usr/local/bin/server"]