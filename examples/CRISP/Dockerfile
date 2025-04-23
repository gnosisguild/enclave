FROM ubuntu:22.04

# Set environment variables
ENV RUST_VERSION=1.85.1 \
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
    wget \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g corepack concurrently wait-on \
    && corepack enable

# Create user
ENV USERNAME=devuser
ENV USER_UID=1000
ENV USER_GID=1000
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

USER $USERNAME
WORKDIR /home/$USERNAME

# Install Rust and add it to PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_VERSION}
ENV PATH="/home/${USERNAME}/.cargo/bin:${PATH}"
RUN rustup component add rustfmt clippy

ENV SHELL=/bin/bash

ENV PATH="/home/${USERNAME}/.foundry/bin:${PATH}"
RUN curl -L https://foundry.paradigm.xyz | bash
RUN foundryup

ENV PATH="/home/${USERNAME}/.risc0/bin:${PATH}"
RUN curl -L https://risczero.com/install | bash
RUN rzup install rust 1.81.0 
RUN rzup install cpp 2024.1.5 
RUN rzup install r0vm 2.0.0
RUN rzup install cargo-risczero 2.0.0

WORKDIR /app/examples/CRISP

RUN sudo chown -R $USERNAME:$USERNAME /app
RUN pnpm config set store-dir "/home/${USERNAME}/.pnpm-store"
USER root
RUN wget https://github.com/mikefarah/yq/releases/download/v4.45.1/yq_linux_386.tar.gz -O - |\
  tar xz && mv yq_linux_386 /usr/bin/yq
USER $USERNAME
ENTRYPOINT ["/bin/bash"]
