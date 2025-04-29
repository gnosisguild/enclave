#!/usr/bin/env bash

set -euo pipefail

PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"

enclave wallet set --name ag --private-key "$PRIVATE_KEY" 

enclave nodes up -v
