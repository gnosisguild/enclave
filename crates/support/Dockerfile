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

COPY . /app

WORKDIR /app

CMD ["/bin/bash"]

