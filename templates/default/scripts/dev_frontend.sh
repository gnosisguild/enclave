#!/usr/bin/env bash

set -euo pipefail

cd client && (export $(enclave print-env --vite --chain hardhat) && pnpm dev)
