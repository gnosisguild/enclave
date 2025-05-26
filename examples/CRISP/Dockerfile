FROM ubuntu:22.04

ARG NOIR_VERSION=1.0.0-beta.6
ARG BB_VERSION=0.87.2
ARG FOUNDRY_VERSION=0.1.0
ARG RISCZERO_VERSION=2.0.0
ARG CPP_VERSION=2024.1.5

# Set environment variables
ENV RUST_VERSION=1.85.0 \
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
    && corepack enable \
    && corepack prepare pnpm@10.7.1 --activate

# Create user
ENV USERNAME=devuser
ARG USER_UID=1000
ARG USER_GID=1000

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

# Install Noir development tools and add it to PATH
ENV SHELL=/bin/bash
RUN curl -L https://raw.githubusercontent.com/noir-lang/noirup/refs/heads/main/install | bash
RUN echo 'export NARGO_HOME="$HOME/.nargo"' >> ~/.bashrc
RUN echo 'export PATH="$PATH:$NARGO_HOME/bin"' >> ~/.bashrc
RUN . ~/.bashrc && /home/${USERNAME}/.nargo/bin/noirup --version ${NOIR_VERSION}

# Install Barretenberg proving backend and add it to PATH
ENV SHELL=/bin/bash
RUN mkdir -p /home/${USERNAME}/.bb
RUN curl -fsSL https://raw.githubusercontent.com/AztecProtocol/aztec-packages/master/barretenberg/bbup/bbup -o /home/${USERNAME}/.bb/bbup
RUN chmod 755 /home/${USERNAME}/.bb/bbup
RUN echo 'export PATH="${HOME}/.bb:${PATH}"' >> ~/.bashrc
RUN . ~/.bashrc && /home/${USERNAME}/.bb/bbup -v ${BB_VERSION}

ENV PATH="/home/${USERNAME}/.foundry/bin:${PATH}"
RUN curl -L https://foundry.paradigm.xyz | bash
RUN foundryup

ENV PATH="/home/${USERNAME}/.risc0/bin:${PATH}"
RUN curl -L https://risczero.com/install | bash
RUN rzup install rust ${RUST_VERSION}
RUN rzup install cpp ${CPP_VERSION} 
RUN rzup install r0vm ${RISCZERO_VERSION}
RUN rzup install cargo-risczero ${RISCZERO_VERSION}

RUN sudo wget https://github.com/mikefarah/yq/releases/download/v4.45.1/yq_linux_amd64.tar.gz -O - |\
  tar xz && sudo mv yq_linux_amd64 /usr/bin/yq

RUN sudo mkdir -p /app && sudo chown -R $USERNAME:$USERNAME /app

WORKDIR /app/examples/CRISP

RUN pnpm config set store-dir "/home/${USERNAME}/.pnpm-store"
RUN pnpm --version

ENTRYPOINT ["/bin/bash"]
