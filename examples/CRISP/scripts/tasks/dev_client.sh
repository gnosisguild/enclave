#!/usr/bin/env bash

set -euo pipefail

sleep 4

(cd ./apps/client && pnpm dev)
