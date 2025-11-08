#!/usr/bin/env bash

set -euo pipefail

# Ensure we're in the right directory
cd circuits

if ! command -v nargo >/dev/null 2>&1
then
    echo "nargo could not be found"
    exit 0 # exiting 0 so that other scripts are not affected
fi

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
