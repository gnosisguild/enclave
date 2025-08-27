#!/usr/bin/env bash
set -euo pipefail

cd packages/circuits
nargo test --workspace

echo "Noir circuits tested successfully"