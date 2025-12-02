#!/usr/bin/env bash

set -e

export CARGO_INCREMENTAL=1

(cd ./server && rm -rf database && cargo run --bin server)
