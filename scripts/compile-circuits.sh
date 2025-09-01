#!/usr/bin/env bash

set -euo pipefail

# Ensure we're in the right directory
cd packages/circuits

# Checking circuit format
echo "Checking circuit format..."
if ! (nargo fmt --check); then
    echo "Error: Circuit format check failed"
    exit 1
fi

# Compile the circuit
echo "Compiling Noir circuit..."
if ! (nargo compile --workspace); then
    echo "Error: Noir circuit compilation failed"
    exit 1
fi

echo "Noir circuits compiled successfully"