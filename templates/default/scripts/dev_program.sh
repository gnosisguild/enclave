#!/usr/bin/env bash

set -euo pipefail
echo "enclave rev = $(enclave rev)"
echo "Waiting on ciphernodes to be ready..."
pnpm wait-on file:/tmp/enclave_ciphernodes_ready && enclave program start
