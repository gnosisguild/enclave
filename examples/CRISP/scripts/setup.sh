#!/usr/bin/env bash

set -euxo pipefail

# This is all stuff that has to happen after the source code is mounted 
# TOOD: perhaps we can try and move more of this to the dockerfile build process
# Eg. copy package.json and Cargo.toml and then try to build out dependencies however this is relatively complex
(cd /app && find . -name "node_modules" -type d -prune -exec rm -rf {} + && pnpm install)
echo "evm"
(cd /app/packages/evm && pnpm compile)
echo "ciphernode"
(cd /app/packages/ciphernode && cargo build && cargo install --path ./enclave)
echo "risc0"
(cd risc0 && cargo build)
echo "server"
(cd server && [[ ! -f .env ]] && cp .env.example .env; cargo check)
echo "web-rust"
(cd web-rust && cargo check)
