#!/usr/bin/env bash

set -euo pipefail

echo "Waiting on ciphernodes to be ready..."
pnpm wait-on file:/tmp/enclave_ciphernodes_ready
echo "Ciphernodes are ready!"
enclave program start
