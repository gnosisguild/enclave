#!/usr/bin/env bash

set -e

export CARGO_INCREMENTAL=1

(cd ./server && cargo run --bin server)
