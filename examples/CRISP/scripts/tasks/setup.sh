#!/usr/bin/env bash

set -euo pipefail

# This is all stuff that has to happen after the source code is mounted 
# TOOD: perhaps we can try and move more of this to the dockerfile build process
# Eg. copy package.json and Cargo.toml and then try to build out dependencies however this is relatively complex
(cd /app && git submodule update --init --recursive)
(cd /app && find . -name "node_modules" -type d -prune -exec rm -rf {} + && pnpm install)
echo "evm"
(cd /app/packages/evm && pnpm compile)
echo "ciphernode"
(cd /app/packages/ciphernode && cargo build && cargo install --path ./enclave --force)
echo "risc0"
(cd ./apps/program && cargo build)
echo "server"
(cd ./apps/server && [[ ! -f .env ]] && cp .env.example .env; cargo check)
echo "crisp-wasm-crypto"
(cd ./apps/wasm-crypto && cargo check)
echo "client"
(cd ./apps/client && if [[ ! -f .env ]]; then cp .env.example .env; fi)
