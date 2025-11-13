#!/usr/bin/env bash

set -euo pipefail

TOOLS_DIR=".vercel/cache/tools"
mkdir -p "$TOOLS_DIR"

# NARGO
if [ ! -x "$TOOLS_DIR/nargo" ]; then
  echo "Downloading nargo..."
  NARGO_VERSION="v1.0.0-beta.3"
  curl -L "https://github.com/noir-lang/noir/releases/download/${NARGO_VERSION}/nargo-x86_64-unknown-linux-musl.tar.gz" \
    | tar -xz -C "$TOOLS_DIR"
  chmod +x "$TOOLS_DIR/nargo"
fi

# WASM-PACK
if [ ! -x "$TOOLS_DIR/wasm-pack" ]; then
  echo "Downloading wasm-pack..."
  WASM_PACK_VERSION="v0.13.1"
  WASM_PACK_TAR="wasm-pack-${WASM_PACK_VERSION}-x86_64-unknown-linux-musl.tar.gz"
  curl -L "https://github.com/drager/wasm-pack/releases/download/${WASM_PACK_VERSION}/${WASM_PACK_TAR}" -o wasm-pack.tar.gz
  tar -xzf wasm-pack.tar.gz
  mv "wasm-pack-${WASM_PACK_VERSION}-x86_64-unknown-linux-musl/wasm-pack" "$TOOLS_DIR/wasm-pack"
  chmod +x "$TOOLS_DIR/wasm-pack"
  # Cleanup
  rm -rf "wasm-pack-${WASM_PACK_VERSION}-x86_64-unknown-linux-musl" wasm-pack.tar.gz
fi

# Verify installations
echo "Verifying installations..."
"$TOOLS_DIR/nargo" --version
"$TOOLS_DIR/wasm-pack" --version
echo "✓ Tools installed successfully"