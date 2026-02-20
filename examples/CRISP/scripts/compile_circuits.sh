#!/usr/bin/env bash

set -euo pipefail

# Script runs from examples/CRISP. Enclave circuits are at ../../circuits.
ENCLAVE_CIRCUITS="../../circuits"
CRISP_CIRCUITS="circuits"

echo "Compiling enclave user_data_encryption circuits (dependencies)..."

echo "Compiling user_data_encryption_ct0..."
if ! (cd "$ENCLAVE_CIRCUITS/bin/threshold/user_data_encryption_ct0" && nargo compile); then
    echo "Error: user_data_encryption_ct0 compilation failed"
    exit 1
fi

echo "Compiling user_data_encryption_ct1..."
if ! (cd "$ENCLAVE_CIRCUITS/bin/threshold/user_data_encryption_ct1" && nargo compile); then
    echo "Error: user_data_encryption_ct1 compilation failed"
    exit 1
fi

echo "Compiling user_data_encryption..."
if ! (cd "$ENCLAVE_CIRCUITS/bin/recursive_aggregation/wrapper/threshold/user_data_encryption" && nargo compile); then
    echo "Error: user_data_encryption compilation failed"
    exit 1
fi

echo "Compiling CRISP circuit..."
if ! (cd "$CRISP_CIRCUITS/bin/crisp" && nargo compile); then
    echo "Error: CRISP circuit compilation failed"
    exit 1
fi

echo "Compiling fold circuit (verifies user_data_encryption + crisp)..."
if ! (cd "$CRISP_CIRCUITS/bin/fold" && nargo compile); then
    echo "Error: Fold circuit compilation failed"
    exit 1
fi

# Generate verifier from fold circuit (on-chain proof verifies the folded proof)
echo "Generating fold Verifier Key..."
if ! bb write_vk -b "$CRISP_CIRCUITS/bin/fold/target/crisp_fold.json" -o "$CRISP_CIRCUITS/bin/fold/target" --oracle_hash keccak; then
    echo "Error: Failed to generate fold Verifier Key"
    exit 1
fi

echo "Generating Solidity Verifier..."
if ! bb write_solidity_verifier -k "$CRISP_CIRCUITS/bin/fold/target/vk" -o "$CRISP_CIRCUITS/bin/fold/target/CRISPVerifier.sol"; then
    echo "Error: Failed to generate Solidity Verifier"
    exit 1
fi

echo "Copying Solidity Verifier to contracts folder..."
if ! cp "$CRISP_CIRCUITS/bin/fold/target/CRISPVerifier.sol" packages/crisp-contracts/contracts/CRISPVerifier.sol; then
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

echo "Formatting CRISPVerifier.sol with Prettier..."
if pnpm exec prettier --write packages/crisp-contracts/contracts/CRISPVerifier.sol 2>/dev/null; then
    echo "Prettier formatting complete"
else
    echo "Warning: Prettier formatting skipped (run pnpm install from repo root if needed)"
fi

echo "Noir setup completed successfully"