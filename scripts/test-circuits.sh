#!/usr/bin/env bash
set -euo pipefail

cd circuits/lib
nargo test --workspace

echo "Noir circuits tested successfully"