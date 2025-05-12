#!/usr/bin/env bash

set -euo pipefail

export CARGO_INCREMENTAL=1

(cd ./apps/server && cargo run --bin server)
