# Stage 1: Build
FROM rust:1.81 AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin p2p_test

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends iptables ca-certificates && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/p2p_test /app/
COPY net/tests/entrypoint.sh /app/
RUN chmod +x /app/entrypoint.sh

ENTRYPOINT ["/app/entrypoint.sh"]
