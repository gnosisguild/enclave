#!/bin/bash

# Check if wasm-pack is installed
if ! command -v wasm-pack >/dev/null 2>&1; then
    echo 'Error: wasm-pack is not installed. Please install it by running:'
    echo 'cargo install wasm-pack'
    exit 1
fi

# Build WASM package if it doesn't exist
if [ ! -f libs/wasm/pkg/wasm_crypto.js ]; then
    echo 'Building WASM package...'
    cd wasm && wasm-pack build --target web --release --out-dir ../libs/wasm/pkg
else
    echo 'WASM package already exists'
fi
