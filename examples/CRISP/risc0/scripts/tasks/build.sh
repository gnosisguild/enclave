#!/usr/bin/env bash

set -e

(cd evm && pnpm compile)
(cd web-rust && cargo build)
(cd risc0 && cargo build)
(cd server && cargo build)
(cd client && pnpm build)
