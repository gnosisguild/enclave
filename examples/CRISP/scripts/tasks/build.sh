#!/usr/bin/env bash

set -e

(cd /app/packages/evm && pnpm compile)
(cd ./apps/wasm-crypto && cargo build --locked)
(cd ./apps/program && cargo build --locked)
(cd ./apps/server && cargo build --locked)
(cd ./apps/client && pnpm build)
