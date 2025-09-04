#!/usr/bin/env bash

set -euo pipefail

wait-on tcp:8545 && \
  (cd ../../packages/enclave-contracts && \
    rm -rf deployments/localhost && \
    pnpm deploy:mocks --network localhost)
