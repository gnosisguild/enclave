#!/usr/bin/env sh
set -eu  # Exit immediately if a command exits with a non-zero status
echo ""
echo "PREBUILDING BINARIES..."
echo ""
cd ../../crates && cargo build --bin fake_encrypt --bin enclave --bin pack_e3_params;
echo ""
echo "FINISHED PREBUILDING BINARIES"
echo ""
