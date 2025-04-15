#!/usr/bin/env bash

set -euo pipefail

sleep 3

(cd ./apps/server && cargo run --bin server)
