FROM ubuntu:22.04

# Set environment variables
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/root/.risc0/bin:$PATH \
    RUST_VERSION=1.81.0 \
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

# Install Node via NVM
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
RUN touch "${BASH_ENV}"
RUN echo '. "${BASH_ENV}"' >> ~/.bashrc
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.2/install.sh | PROFILE="${BASH_ENV}" bash
RUN echo node > .nvmrc
RUN nvm install 22

# Install Rust 1.81.0
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_VERSION} \
    && rustup component add rustfmt clippy

# Install RISC0
RUN curl -L https://risczero.com/install | bash
RUN rzup install

RUN npm install -g corepack

# Create a working directory
WORKDIR /app

RUN corepack enable

# Set entrypoint
ENTRYPOINT ["/bin/bash"]
