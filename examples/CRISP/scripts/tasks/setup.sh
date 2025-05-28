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
(cd /app && cargo build -p e3-cli && cargo install --path crates/cli)
echo "program"
(cd ./apps/program && cargo build --bin crisp-program)
echo "server"
(cd ./apps/server && [[ ! -f .env ]] && cp .env.example .env; cargo build --bin cli && cargo build --bin server)
echo "crisp-wasm-crypto"
(cd ./apps/wasm-crypto && cargo check)
echo "client"
(cd ./apps/client && if [[ ! -f .env ]]; then cp .env.example .env; fi)
echo "noir"
(cd ./circuits && nargo compile)
# Copy circuits compilation artifacts to public client app folder
mkdir -p ./apps/client/libs/noir
cp -r ./circuits/target/* ./apps/client/libs/noir/
# Generate the Verifier & copy to the contracts folder
bb write_vk -b ./circuits/target/*.json -o ./circuits/target --oracle_hash keccak
bb write_solidity_verifier -k ./circuits/target/vk -o ./circuits/target/CRISPVerifier.sol
cp ./circuits/target/CRISPVerifier.sol ./contracts/CRISPVerifier.sol
