#!/usr/bin/env bash

set -euo pipefail

pnpm wait-on http://localhost:8545 && enclave program start
