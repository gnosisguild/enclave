FROM ubuntu:22.04

# Set environment variables
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.85.1 \
    RISC0_DEV_MODE=1 \
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
    sudo \
    && rm -rf /var/lib/apt/lists/*

# Set up non-root user with sudo privileges
ARG USERNAME=devuser
ARG USER_UID=1000
ARG USER_GID=1000

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

# Install Node.js directly through apt
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g corepack concurrently wait-on \
    && corepack enable

# Rust - install globally
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_VERSION} \
    && rustup component add rustfmt clippy

# RISC0 - install globally
ENV PATH=/usr/local/risc0/bin:$PATH
RUN curl -L https://risczero.com/install | bash
ENV PATH=~/.risc0/bin:$PATH
RUN rzup install rust 1.81.0 
RUN rzup install cpp 2024.1.5 
RUN rzup install r0vm 1.2.0 
RUN rzup install cargo-risczero 1.2.0 

# Foundry - install globally
ENV PATH=/usr/local/foundry/bin:$PATH
RUN curl -L https://foundry.paradigm.xyz | bash
RUN foundryup

# Make sure tools are accessible to non-root user
RUN mkdir -p /usr/local/risc0 /usr/local/foundry \
    && chown -R $USERNAME:$USERNAME /usr/local/cargo /usr/local/rustup /usr/local/risc0 /usr/local/foundry

# Set working directory
WORKDIR /app
RUN chown -R $USERNAME:$USERNAME /app

# Switch to non-root user
USER $USERNAME

ENTRYPOINT ["/bin/bash"]
