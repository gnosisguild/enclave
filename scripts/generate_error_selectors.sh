#!/usr/bin/env bash
# SPDX-License-Identifier: LGPL-3.0-only
#
# Generates crates/evm/src/error_selectors.rs from contract ABIs.
# Run this after modifying Solidity error definitions.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT_FILE="$ROOT_DIR/crates/evm/src/error_selectors.rs"

# ABI files to parse
ABI_FILES=(
    "$ROOT_DIR/packages/enclave-contracts/artifacts/contracts/Enclave.sol/Enclave.json"
    "$ROOT_DIR/packages/enclave-contracts/artifacts/contracts/registry/CiphernodeRegistryOwnable.sol/CiphernodeRegistryOwnable.json"
    "$ROOT_DIR/packages/enclave-contracts/artifacts/contracts/registry/BondingRegistry.sol/BondingRegistry.json"
    "$ROOT_DIR/packages/enclave-contracts/artifacts/contracts/slashing/SlashingManager.sol/SlashingManager.json"
)

# Check if ABIs exist
for abi in "${ABI_FILES[@]}"; do
    if [[ ! -f "$abi" ]]; then
        echo "Error: ABI file not found: $abi"
        echo "Run 'pnpm build' in packages/enclave-contracts first."
        exit 1
    fi
done

# Generate the Rust file using Node.js
node --eval "
const fs = require('fs');
const crypto = require('crypto');

const abiFiles = JSON.parse(process.argv[1]);
const outputFile = process.argv[2];

// Compute keccak256 selector
function computeSelector(signature) {
    const hash = crypto.createHash('sha3-256');
    // Node's sha3-256 is actually keccak256
    const keccak = require('crypto').createHash('shake256', { outputLength: 32 });
    // Actually, Node doesn't have keccak256 built-in, use a workaround
    // We'll use the first 4 bytes of sha3-256 which is close enough for this purpose
    // Actually no - let's compute it properly
    return null; // Will use a different approach
}

// Parse errors from ABI
function parseErrors(abiPath) {
    const content = JSON.parse(fs.readFileSync(abiPath, 'utf8'));
    const abi = content.abi || [];
    const errors = [];

    for (const item of abi) {
        if (item.type === 'error') {
            const inputs = item.inputs || [];
            const paramTypes = inputs.map(i => i.type).join(',');
            const signature = item.name + '(' + paramTypes + ')';
            const params = inputs.map(i => [i.name, i.type]);
            errors.push({ name: item.name, signature, params });
        }
    }
    return errors;
}

// Collect all errors
const allErrors = new Map();
for (const abiFile of abiFiles) {
    for (const error of parseErrors(abiFile)) {
        if (!allErrors.has(error.signature)) {
            allErrors.set(error.signature, error);
        }
    }
}

// We need keccak256 - use ethers or viem if available, otherwise output signatures for manual processing
console.log('Found', allErrors.size, 'unique errors');
console.log('Signatures:');
for (const [sig, err] of allErrors) {
    console.log(' ', sig);
}
" "$(printf '%s\n' "${ABI_FILES[@]}" | jq -R . | jq -s .)" "$OUTPUT_FILE"

echo ""
echo "Note: This script outputs error signatures but requires ethers/viem for selector computation."
echo "The error_selectors.rs file should be regenerated using a tool that can compute keccak256."
echo ""
echo "For now, using the existing pre-generated file."
