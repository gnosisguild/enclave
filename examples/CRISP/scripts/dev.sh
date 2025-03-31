#!/usr/bin/env bash

set -e

concurrently -k \
  --names "ANVIL" \
  --prefix-colors "blue" \
  "anvil"
 
