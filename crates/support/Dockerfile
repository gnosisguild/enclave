ARG RUST_VERSION=1.86.0
ARG RISC0_VERSION=2.1.0
ARG SKIP_SOLIDITY=0
FROM rust:${RUST_VERSION}-slim-bookworm AS base-dev

ENV CARGO_HOME=/usr/local/cargo
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    pkg-config \
    git \
    libssl-dev \
    curl \
    sudo \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*
    
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

ENV SHELL=/bin/bash

ENV PATH="/home/${USERNAME}/.risc0/bin:${PATH}"
RUN curl -L https://risczero.com/install | bash
RUN rzup install rust ${RUST_VERSION}
RUN rzup install r0vm ${RISC0_VERSION}
RUN rzup install cargo-risczero ${RISC0_VERSION}

ENV PATH="/home/${USERNAME}/.foundry/bin:${PATH}"
RUN curl -L https://foundry.paradigm.xyz | bash
RUN foundryup

COPY . /app

RUN sudo chown -R $USERNAME:$USERNAME /app

WORKDIR /app

CMD ["/bin/bash"]
