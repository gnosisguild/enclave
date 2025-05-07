# Stage 1: Build
FROM rust:1.81 AS builder

WORKDIR /app
COPY . .

WORKDIR /app/ciphernode
RUN cargo build --release --bin p2p_test

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends iptables ca-certificates && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/ciphernode/target/release/p2p_test .

ENTRYPOINT ["/app/p2p_test"]
