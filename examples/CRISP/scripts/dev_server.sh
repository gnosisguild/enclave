#!/usr/bin/env bash

set -euo pipefail

(cd ./server && cargo run --bin server)
