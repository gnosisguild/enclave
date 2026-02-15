#!/usr/bin/env bash

set -euo pipefail

echo "CLIENT SCRIPT RUNNING..."

(cd ./client && if [[ ! -f .env ]]; then cp .env.example .env; fi && pnpm dev-static)
