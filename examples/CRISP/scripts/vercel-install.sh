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

# Verify installations
echo "Verifying installations..."
"$TOOLS_DIR/nargo" --version
echo "✓ Tools installed successfully"