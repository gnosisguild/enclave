#!/usr/bin/env bash

set -euo pipefail

(cd ./apps/server && cargo run --bin server)
