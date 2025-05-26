#!/usr/bin/env bash

set -euo pipefail

echo "noir"
# Ensure we're in the right directory
cd /app/examples/CRISP

# Compile the circuit
echo "Compiling Noir circuit..."
if ! (cd circuits && nargo compile); then
    echo "Error: Noir circuit compilation failed"
    exit 1
fi

# Create the public circuits directory
echo "Creating public circuits directory..."
mkdir -p apps/client/public/circuits

# Copy the compiled artifacts
echo "Copying circuit artifacts..."
if ! cp -r circuits/target/* apps/client/public/circuits/; then
    echo "Error: Failed to copy circuit artifacts"
    exit 1
fi

echo "Noir setup completed successfully"