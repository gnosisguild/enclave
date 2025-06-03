#!/usr/bin/env bash

set -euo pipefail

# Ensure we're in the right directory
cd "$(dirname "$0")/.."

# Run the compilation script
./scripts/tasks/compile_circuits.sh