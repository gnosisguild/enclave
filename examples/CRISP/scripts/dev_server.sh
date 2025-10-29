#!/usr/bin/env bash

set -e

export CARGO_INCREMENTAL=1

(cd ./server && cargo build --bin server && cargo build --bin cli && cargo run --bin server)
