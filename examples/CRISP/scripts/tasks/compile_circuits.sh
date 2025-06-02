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
mkdir -p apps/client/libs/noir

# Copy the compiled artifacts
echo "Copying circuit artifacts..."
if ! cp -r circuits/target/* apps/client/libs/noir/; then
    echo "Error: Failed to copy circuit artifacts"
    exit 1
fi

# Generate the Verifier
echo "Generating Verifier Key..."
if ! bb write_vk -b circuits/target/*.json -o circuits/target --oracle_hash keccak; then
    echo "Error: Failed to generate Verifier Key"
    exit 1
fi

echo "Generating Solidity Verifier..."
if ! bb write_solidity_verifier -k circuits/target/vk -o circuits/target/CRISPVerifier.sol; then
    echo "Error: Failed to generate Solidity Verifier"
    exit 1
fi

# Copy the Solidity Verifier to the contracts folder
echo "Copying Solidity Verifier to contracts folder..."
if ! cp circuits/target/CRISPVerifier.sol contracts/CRISPVerifier.sol; then
    echo "Error: Failed to copy Solidity Verifier to contracts folder"
    exit 1
fi

echo "Noir setup completed successfully"