#!/usr/bin/env bash

set -e

(cd evm && pnpm compile)
(cd risc0 && cargo build)
(cd server && cargo build)
(cd client && pnpm build)
