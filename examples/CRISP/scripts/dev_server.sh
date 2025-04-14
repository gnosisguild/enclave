#!/usr/bin/env bash

set -euo pipefail

sleep 3

(cd ./server && cargo run --bin server)
