#!/usr/bin/env bash

wait-on tcp:8545 && \
  (cd /app/packages/evm && \
    rm -rf deployments/localhost && \
    pnpm deploy:mocks --network hardhat)
