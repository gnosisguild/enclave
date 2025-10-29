#!/usr/bin/env bash

set -euo pipefail

sleep 4

(cd ./client && pnpm dev-static)
