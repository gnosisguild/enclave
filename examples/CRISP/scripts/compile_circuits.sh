#!/usr/bin/env bash

set -euo pipefail

echo "noir"

# Compile the circuit
echo "Compiling Noir circuit..."
if ! (cd circuits && nargo compile); then
    echo "Error: Noir circuit compilation failed"
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
if ! cp circuits/target/CRISPVerifier.sol packages/crisp-contracts/contracts/CRISPVerifier.sol; then
    echo "Error: Failed to copy Solidity Verifier to contracts folder"
    exit 1
fi

# Add the correct license header
echo "Adding license header to CRISPVerifier.sol..."
LICENSE_HEADER="// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE."
# Remove the first 2 lines (Apache license and copyright) and prepend our license header
TEMP_FILE=$(mktemp)
{
    echo "$LICENSE_HEADER"
    tail -n +3 packages/crisp-contracts/contracts/CRISPVerifier.sol
} > "$TEMP_FILE"
mv "$TEMP_FILE" packages/crisp-contracts/contracts/CRISPVerifier.sol

echo "Noir setup completed successfully"