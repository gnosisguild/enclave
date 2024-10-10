#!/usr/bin/env bash

RUSTFLAGS="-A warnings" RUST_LOG=info cargo run --bin enclave -- "$@"
