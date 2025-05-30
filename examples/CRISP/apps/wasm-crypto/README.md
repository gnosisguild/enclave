# CRISP-Web

A Rust WebAssembly (WASM) implementation of Fully Homomorphic Encryption (FHE) for secure voting systems.

## Features

- BFV encryption scheme implementation
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
wasm-pack build --target web --release
```


## Running Tests

Run the tests:
```
wasm-pack test --node --release
```
