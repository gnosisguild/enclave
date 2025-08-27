#!/usr/bin/env bash

set -e

export CARGO_INCREMENTAL=1

echo "pnpm install"
(cd ../../ && pnpm install --frozen-lockfile)
echo "evm"
(cd ../../packages/evm && pnpm compile)
echo "server"
(cd ./server && [[ ! -f .env ]] && cp .env.example .env; [[ ! -f ../.env ]] && cp .env.example ../.env; cargo build --locked --bin cli && cargo build --locked --bin server)
echo "crisp-wasm-crypto"
(cd ./wasm-crypto && cargo check)
echo "client"
(cd ./client && if [[ ! -f .env ]]; then cp .env.example .env; fi)
echo "ciphernode"
(cd ../../ && cargo build --locked -p e3-cli && cargo install --locked --path crates/cli)
echo "Skipping circuit compilation - using pre-compiled circuits"