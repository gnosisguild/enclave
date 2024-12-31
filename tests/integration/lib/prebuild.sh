#!/usr/bin/env sh
echo ""
echo "PREBUILDING BINARIES..."
echo ""
cd packages/ciphernode && cargo build --bin fake_encrypt --bin enclave --bin pack_e3_params;
echo ""
echo "FINISHED PREBUILDING BINARIES"
echo ""
