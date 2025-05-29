# CRISP-Web

A Rust WebAssembly (WASM) implementation of Fully Homomorphic Encryption (FHE) with Zero-Knowledge Proofs (ZKP) for secure voting systems.

## Features

- BFV encryption scheme implementation
- Zero-Knowledge Proofs using Halo2
- WASM integration for browser-based encryption
- Greco protocol implementation for input validation

## Prerequisites

- Rust (latest stable version)
- wasm-pack
- Node.js (for running tests)

## Installation

1. Install wasm-pack
`cargo install wasm-pack`

## Building

Build the WebAssembly package:
```
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--max-memory=4294967296" \
rustup run nightly-2024-08-02 \
wasm-pack build --target web --release --verbose \
  -Z build-std=std,panic_abort
```
> Note: wasm-pack doesn't pass the .cargo/config.toml file to the build process, so we need to manually add the target-feature flags. See [halo2 WASM environment setup](https://zcash.github.io/halo2/user/wasm-port.html#rust-and-wasm-environment-setup) for more information about the target-feature flags.

Also, the `--verbose` is necessary to make the build succeed for some reason.


## Running Tests

Run the tests:
```
RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--max-memory=4294967296" \
rustup run nightly-2024-08-02 \
wasm-pack test --node --release \
  -Z build-std=std,panic_abort
```
