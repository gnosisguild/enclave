#!/usr/bin/env bash

wait-on tcp:8545 && (cd ./enclave/packages/evm && rm -rf deployments/localhost && yarn deploy:mocks --network hardhat)
