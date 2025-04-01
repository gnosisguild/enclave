#!/usr/bin/env bash

set -e

concurrently \
  --names "ANVIL,DEPLOY" \
  --prefix-colors "blue,green" \
  "anvil" \
  "./scripts/evm_deploy.sh && ./scripts/risc0_deploy.sh"


