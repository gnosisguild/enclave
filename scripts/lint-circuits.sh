#!/usr/bin/env bash
set -euo pipefail

# Check if nargo command exists
if ! command -v nargo >/dev/null 2>&1; then
    echo "nargo command not found, skipping circuit checks"
    exit 0
fi

# Ensure we're in the right directory.
cd circuits

# Checking circuit format.
echo "Checking circuit format..."
if ! (nargo fmt --check); then
    echo "Error: Circuit format check failed"
    exit 1
fi

# Check/validate the circuit libraries.
echo "Checking Noir circuit libraries..."
if ! (nargo check --workspace); then
    echo "Error: Noir circuit check failed"
    exit 1
fi

echo "Noir circuits checked successfully"
