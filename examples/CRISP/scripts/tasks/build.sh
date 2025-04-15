#!/usr/bin/env bash

set -e

(cd evm && pnpm compile)
(cd ./apps/wasm-crypto && cargo build)
(cd ./apps/program && cargo build)
(cd ./apps/server && cargo build)
(cd ./apps/client && pnpm build)
