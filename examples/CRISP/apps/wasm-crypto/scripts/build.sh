#!/usr/bin/env bash

RUSTFLAGS="-C target-feature=+atomics,+bulk-memory" pnpm wasm-pack build --target web --out-dir pkg-multi --features parallel -Z build-std=panic_abort,std
