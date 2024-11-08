FROM rust:1.81 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release --bin discover --bin register --bin rendezvous

# Switch to Alpine for the runtime image
FROM alpine:3.19

# Install necessary runtime dependencies and networking tools
RUN apk add --no-cache \
    libgcc \
    iputils \
    curl \
    net-tools \
    libc6-compat \
    file \
    binutils

COPY --from=builder /usr/src/app/target/release/discover /usr/local/bin/
COPY --from=builder /usr/src/app/target/release/register /usr/local/bin/
COPY --from=builder /usr/src/app/target/release/rendezvous /usr/local/bin/

