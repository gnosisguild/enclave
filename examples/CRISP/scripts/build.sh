#!/usr/bin/env bash

set -e

(cargo build --locked)
(cd ../../packages/enclave-contracts && pnpm compile)
(cd ./client && pnpm build)
