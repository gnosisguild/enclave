#!/usr/bin/env sh

cd packages/ciphernode && RUSTFLAGS="-A warnings" cargo build --bin fake_encrypt --bin enclave --bin pack_e3_params;
