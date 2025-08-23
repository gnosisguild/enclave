#!/usr/bin/env bash

set -euo pipefail

export CARGO_INCREMENTAL=1

# This is all stuff that has to happen after the source code is mounted 
# TOOD: perhaps we can try and move more of this to the dockerfile build process
# Eg. copy package.json and Cargo.toml and then try to build out dependencies however this is relatively complex
echo "pnpm install"
(cd /app && pnpm install --frozen-lockfile)
echo "evm"
(cd /app/packages/evm && pnpm compile)
echo "ciphernode"
(cd /app && cargo build --locked -p e3-cli && cargo install --locked --path crates/cli)
echo "server"
(cd ./server && [[ ! -f .env ]] && cp .env.example ../.env; cargo build --locked --bin cli && cargo build --locked --bin server)
echo "crisp-wasm-crypto"
(cd ./wasm-crypto && cargo check)
echo "client"
(cd ./client && if [[ ! -f .env ]]; then cp .env.example .env; fi)
# echo "noir"
# ./scripts/tasks/compile_circuits.sh
echo "Skipping circuit compilation - using pre-compiled circuits"
