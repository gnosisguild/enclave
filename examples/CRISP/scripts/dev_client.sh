#!/usr/bin/env bash

set -euo pipefail

echo "CLIENT SCRIPT RUNNING..."

(cd ./client && pnpm dev-static)
