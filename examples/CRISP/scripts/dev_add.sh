#!/usr/bin/env bash

set -euo pipefail

# wait until nodes are up
sleep 3

cd /app && /app/examples/CRISP/local_testnet/add_ciphernodes.sh
