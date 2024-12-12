#!/usr/bin/env sh

cd packages/ciphernode && cargo build --bin enclave --bin fake_encrypt --bin pack_e3_params
