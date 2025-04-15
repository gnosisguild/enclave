#!/usr/bin/env bash

set -e

(cd evm && pnpm compile)
(cd ./apps/wasm-src && cargo build)
(cd ./apps/risc0 && cargo build)
(cd ./apps/server && cargo build)
(cd ./apps/client && pnpm build)
