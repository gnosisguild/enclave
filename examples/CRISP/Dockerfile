# ────────────────────────────────────────────────────────────────────────────────
#   Base image & build-time versions
# ────────────────────────────────────────────────────────────────────────────────
FROM ubuntu:24.04

ARG NOIR_VERSION=1.0.0-beta.3
ARG BB_VERSION=0.82.2
ARG FOUNDRY_VERSION=0.1.0
ARG RISCZERO_VERSION=2.0.0
ARG CPP_VERSION=2024.1.5
ARG RUST_VERSION=1.85.0

# ────────────────────────────────────────────────────────────────────────────────
#   OS packages
# ────────────────────────────────────────────────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential cmake curl git libssl-dev pkg-config \
    python3 python3-pip sudo wget jq \
 && rm -rf /var/lib/apt/lists/*

# Node 18 / 22 + pnpm
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
 && apt-get install -y nodejs \
 && npm install -g corepack concurrently wait-on \
 && corepack enable \
 && corepack prepare pnpm@10.7.1 --activate

# ────────────────────────────────────────────────────────────────────────────────
#   Non-root user
# ────────────────────────────────────────────────────────────────────────────────
ARG USERNAME=ubuntu
ARG USER_UID=1000
ARG USER_GID=1000

RUN groupadd --gid $USER_GID $USERNAME \
 && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
 && echo "$USERNAME ALL=(root) NOPASSWD:ALL" > /etc/sudoers.d/$USERNAME \
 && chmod 0440 /etc/sudoers.d/$USERNAME

USER $USERNAME
WORKDIR /home/$USERNAME

# ────────────────────────────────────────────────────────────────────────────────
#   Rust toolchain
# ────────────────────────────────────────────────────────────────────────────────
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
 | sh -s -- -y --default-toolchain ${RUST_VERSION}
ENV PATH="/home/${USERNAME}/.cargo/bin:${PATH}"
RUN rustup component add rustfmt clippy

# ────────────────────────────────────────────────────────────────────────────────
#   Noir / Nargo
# ────────────────────────────────────────────────────────────────────────────────
ENV NARGO_HOME="/home/${USERNAME}/.nargo"
ENV PATH="${PATH}:${NARGO_HOME}/bin"

RUN curl -L https://raw.githubusercontent.com/noir-lang/noirup/refs/heads/main/install | bash \
 && noirup --tag ${NOIR_VERSION}    # installs nargo + backends for requested tag

# ────────────────────────────────────────────────────────────────────────────────
#   Barretenberg CLI (bbup)
# ────────────────────────────────────────────────────────────────────────────────
ENV BB_HOME="/home/${USERNAME}/.bb"
ENV PATH="${PATH}:${BB_HOME}"

RUN curl -L https://raw.githubusercontent.com/AztecProtocol/aztec-packages/refs/heads/master/barretenberg/bbup/install | bash \
 && bbup -v ${BB_VERSION}

# ────────────────────────────────────────────────────────────────────────────────
#   Foundry
# ────────────────────────────────────────────────────────────────────────────────
ENV PATH="/home/${USERNAME}/.foundry/bin:${PATH}"
RUN curl -L https://foundry.paradigm.xyz | bash \
 && foundryup --version ${FOUNDRY_VERSION}

# ────────────────────────────────────────────────────────────────────────────────
#   Risc Zero tool-chain
# ────────────────────────────────────────────────────────────────────────────────
ENV PATH="/home/${USERNAME}/.risc0/bin:${PATH}"

RUN curl -L https://risczero.com/install | bash \
 && rzup install rust ${RUST_VERSION} \
 && rzup install cpp  ${CPP_VERSION} \
 && rzup install r0vm ${RISCZERO_VERSION} \
 && rzup install cargo-risczero ${RISCZERO_VERSION}

# ────────────────────────────────────────────────────────────────────────────────
#   Helpers (yq)  & workspace dir
# ────────────────────────────────────────────────────────────────────────────────
RUN sudo wget -qO- https://github.com/mikefarah/yq/releases/download/v4.45.1/yq_linux_amd64.tar.gz \
 | sudo tar xz -C /usr/local/bin --transform='s|yq_linux_amd64|yq|' --no-same-owner

RUN sudo mkdir -p /app && sudo chown -R $USERNAME:$USERNAME /app
WORKDIR /app

# Pre-set pnpm store path (speeds up mounts)
RUN pnpm config set store-dir "/home/${USERNAME}/.pnpm-store" \
 && pnpm --version

# ────────────────────────────────────────────────────────────────────────────────
#   Default entrypoint
# ────────────────────────────────────────────────────────────────────────────────
ENTRYPOINT ["bash", "-l"]
