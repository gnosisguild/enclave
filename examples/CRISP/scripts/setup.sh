#!/usr/bin/env bash

set -e

export CARGO_INCREMENTAL=1

echo "pnpm install"
(cd ../../ && pnpm install --frozen-lockfile)
echo "evm"
(cd ../../packages/enclave-contracts && pnpm compile)
echo "server"
(cd ./server && [[ ! -f .env ]] && cp .env.example .env; [[ ! -f ../.env ]] && cp .env.example ../.env; cargo build --locked --bin cli && cargo build --locked --bin server)
echo "client"
(cd ./client && if [[ ! -f .env ]]; then cp .env.example .env; fi)
echo "ciphernode"
if [[ ! -f ~/.cargo/bin/enclave ]]; then
  echo "Building and installing enclave CLI..."
  (cd ../../ && cargo build --locked -p e3-cli && cargo install --locked --path crates/cli)
else
  echo "enclave CLI already installed, skipping build"
fi
echo "Skipping circuit compilation - using pre-compiled circuits"