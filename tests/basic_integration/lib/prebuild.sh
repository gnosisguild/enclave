#!/usr/bin/env sh

cd packages/ciphernode && cargo build --bin fake_encrypt --bin enclave --bin pack_e3_params;
