#!/usr/bin/env bash
set -euo pipefail

cd circuits
nargo test --workspace

echo "Noir circuits tested successfully"