#!/usr/bin/env bash

set -e

(cargo build --locked)
(cd ./app/packages/evm && pnpm compile)
(cd ./client && pnpm build)
