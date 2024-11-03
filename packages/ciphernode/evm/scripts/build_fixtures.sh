#!/usr/bin/env bash

set -e

echo "Building fixtures..."

echo "{\"abi\": $(solc --abi tests/fixtures/emit_logs.sol | tail -n 1), \"bin\": \"$(solc --bin tests/fixtures/emit_logs.sol| tail -n 1)\"}" | jq '.' > tests/fixtures/emit_logs.json
