#!/usr/bin/env bash

set -e

DONE=/tmp/.e1

rm -rf $DONE

concurrently -k \
  --names "ANVIL,DEPLOY,CIPHER,AGG,ADD" \
  --prefix-colors "blue,green,red" \
  "anvil" \
  "./scripts/evm_deploy.sh && ./scripts/risc0_deploy.sh && touch $DONE || true" \
  "wait-on file:$DONE && ./scripts/dev_cipher.sh" \
  "wait-on file:$DONE && ./scripts/dev_agg.sh" \
  "wait-on file:$DONE && ./scripts/dev_add.sh"

