# Stage 1: Base image with common dependencies
FROM ubuntu:22.04 AS base
ENV SHELL=/bin/bash
# Create user
ENV USERNAME=devuser
ARG USER_UID=1000
ARG USER_GID=1000

# Install sudo first and create required directories
RUN apt-get update && apt-get install -y \
    sudo \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /etc/sudoers.d

# Now create the user with sudo access
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

# Install common dependencies
RUN apt-get update && apt-get install -y \
    curl \
    git \
    wget \
    && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /app && chown -R $USERNAME:$USERNAME /app

# Stage 2: Rust toolchain
FROM base AS rust-builder
USER $USERNAME
WORKDIR /home/$USERNAME
ENV RUST_VERSION=1.85.0
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_VERSION}
ENV PATH="/home/${USERNAME}/.cargo/bin:${PATH}"
RUN rustup default ${RUST_VERSION}
RUN rustup component add rustfmt clippy
# Install RISC Zero tools
ENV PATH="/home/${USERNAME}/.risc0/bin:${PATH}"
RUN curl -L https://risczero.com/install | bash
RUN rzup install rust 1.85.0 
RUN rzup install cpp 2024.1.5 
RUN rzup install r0vm 2.0.0
RUN rzup install cargo-risczero 2.0.0
RUN cargo --version && rustc --version

# Stage 3: Node.js and JavaScript tooling
FROM base AS node-builder
USER $USERNAME
WORKDIR /home/$USERNAME
# Install Node.js and related tools
RUN sudo apt-get update && sudo apt-get install -y \
    build-essential \
    python3 \
    python3-pip \
    && sudo rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | sudo bash - \
    && sudo apt-get install -y nodejs \
    && sudo npm install -g corepack concurrently wait-on \
    && sudo corepack enable \
    && sudo corepack prepare pnpm@10.7.1 --activate
RUN pnpm config set store-dir "/home/${USERNAME}/.pnpm-store"

# Stage 4: Foundry tools
FROM base AS foundry-builder
USER $USERNAME
WORKDIR /home/$USERNAME
ENV PATH="/home/${USERNAME}/.foundry/bin:${PATH}"
RUN curl -L https://foundry.paradigm.xyz | bash
RUN foundryup

# Stage 5: Final stage combining all tools
FROM base AS final
ENV RUST_VERSION=1.85.0 \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0 \
    SHELL=/bin/bash
USER root
# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    libssl-dev \
    pkg-config \
    python3 \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*
# Install Node.js
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install yq
RUN wget https://github.com/mikefarah/yq/releases/download/v4.45.1/yq_linux_amd64.tar.gz -O - |\
  tar xz && mv yq_linux_amd64 /usr/bin/yq

# Copy Rust toolchain from rust-builder
COPY --from=rust-builder --chown=$USERNAME:$USERNAME /home/$USERNAME/.cargo /home/$USERNAME/.cargo
COPY --from=rust-builder --chown=$USERNAME:$USERNAME /home/$USERNAME/.risc0 /home/$USERNAME/.risc0
ENV PATH="/home/${USERNAME}/.cargo/bin:/home/${USERNAME}/.risc0/bin:${PATH}"

# Switch to devuser but install global npm packages as root
USER root
RUN npm install -g corepack concurrently wait-on \
    && corepack enable \
    && corepack prepare pnpm@10.7.1 --activate

# Switch back to devuser for remaining operations
USER $USERNAME
WORKDIR /home/$USERNAME
RUN pnpm config set store-dir "/home/${USERNAME}/.pnpm-store"
RUN pnpm --version

# Copy Foundry from foundry-builder
COPY --from=foundry-builder --chown=$USERNAME:$USERNAME /home/$USERNAME/.foundry /home/$USERNAME/.foundry
ENV PATH="/home/${USERNAME}/.foundry/bin:${PATH}"

WORKDIR /app/examples/CRISP
ENTRYPOINT ["/bin/bash"]
