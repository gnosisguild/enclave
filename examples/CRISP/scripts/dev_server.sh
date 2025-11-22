#!/usr/bin/env bash

set -e

export CARGO_INCREMENTAL=1

echo "<<................RUNNING SERVER...............>>"


(cd ./server && cargo run --bin server)
