#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "${SCRIPT_DIR}/lib/dev_config.sh"

load_template_dev_config
cd "${TEMPLATE_ROOT}"

echo "Installing dependencies..."
pnpm install --frozen-lockfile

echo "Compiling guest program..."
if [[ ! -f './.enclave/generated/contracts/ImageID.sol' ]]; then
  enclave program compile
fi

build_enclave_circuits_at_setup

echo "Compiling contracts..."
pnpm compile

if [[ ! -f ~/.cargo/bin/enclave ]]; then
  echo "Building and installing enclave CLI..."
  (cd "${ENCLAVE_REPO_ROOT}" && cargo build --locked -p e3-cli && cargo install --locked --path crates/cli)
else
  echo "enclave CLI already installed, skipping build"
fi

echo "Running enclave noir setup..."
enclave noir setup

echo "Template setup complete."
