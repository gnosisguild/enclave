#!/usr/bin/env bash

set -euo pipefail

# wait until nodes are up
sleep 3

cd /app && /app/examples/CRISP/scripts/local_dev/add_ciphernodes.sh
