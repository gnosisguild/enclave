#!/usr/bin/env bash

set -e

# Check if wasm-pack is installed
if ! command -v wasm-pack >/dev/null 2>&1; then
    echo 'Error: wasm-pack is not installed. Please install it by running:'
    echo 'cargo install wasm-pack'
    exit 1
fi

# Function to check if wasm32 target is available
check_wasm_target() {
    rustc --print target-list | grep -q wasm32-unknown-unknown
}

# Function to try installing wasm32 target
install_wasm_target() {
    if command -v rustup >/dev/null 2>&1; then
        echo "Installing wasm32-unknown-unknown target..."
        rustup target add wasm32-unknown-unknown
        return 0
    else
        echo "Rustup not found. Cannot install wasm32-unknown-unknown target automatically."
        echo "For Nix users: Please ensure your development environment includes the wasm32 target."
        echo "You may need to add it to your shell.nix or flake.nix configuration."
        return 1
    fi
}

# Check if wasm32 target is available
if ! check_wasm_target; then
    echo "wasm32-unknown-unknown target not found"
    
    # Try to install it
    if ! install_wasm_target; then
        exit 1
    fi
fi

# Build WASM package if it doesn't exist
if [ ! -f libs/wasm/pkg/wasm_crypto.js ]; then
    echo 'Building WASM package...'
    cd wasm && wasm-pack build --target web --release --out-dir ../libs/wasm/pkg
else
    echo 'WASM package already exists'
fi
