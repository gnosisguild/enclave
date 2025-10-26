# syntax=docker/dockerfile:1-labs
FROM rust:1.86 AS builder

WORKDIR /app

# Copy workspace files and all crates EXCEPT net
COPY --exclude=crates/net . .
COPY ./crates/net/Cargo.toml ./crates/net/Cargo.toml
RUN mkdir -p ./crates/net/src/bin && \
    echo "fn main() {}" > ./crates/net/src/main.rs && \
    echo "fn main() {}" > ./crates/net/src/bin/p2p_test.rs

# Build dependencies
RUN cargo build --locked --bin p2p_test

# Copy full net crate
COPY ./crates/net ./crates/net

RUN cargo build --locked --bin p2p_test

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    iptables ca-certificates iputils-ping netcat-traditional && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/debug/p2p_test .
