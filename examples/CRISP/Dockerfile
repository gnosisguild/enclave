FROM ubuntu:22.04

# Set environment variables
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/root/.risc0/bin:/root/.foundry/bin:$PATH \
    RUST_VERSION=1.85.1 \
    RISC0_DEV_MODE=1 \
    BASH_ENV=/root/.bash_env \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0


# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    curl \
    git \
    libssl-dev \
    pkg-config \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Node via NVM
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN touch "${BASH_ENV}"
RUN echo '. "${BASH_ENV}"' >> ~/.bashrc
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.2/install.sh | PROFILE="${BASH_ENV}" bash
RUN echo node > .nvmrc
RUN nvm install 22
RUN npm install -g corepack concurrently wait-on
RUN corepack enable

# Rust 
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_VERSION} \
    && rustup component add rustfmt clippy

# RISC0
RUN curl -L https://risczero.com/install | bash
RUN rzup install rust 1.81.0 # set this to the desired risc0 rust toolchain.
RUN rzup install cpp 2024.1.5 # only required if the guest uses any C++ code or rust crates that bind to C++ code.
RUN rzup install r0vm 1.2.0 # should be the same version as cargo-risczero and risc0-zkvm crate
RUN rzup install cargo-risczero 1.2.0 # should be the same version as cargo-risczero and risc0-zkvm crate

# Foundry
RUN curl -L https://foundry.paradigm.xyz | bash
RUN foundryup

WORKDIR /app

ENTRYPOINT ["/bin/bash"]
