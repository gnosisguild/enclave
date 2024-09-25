#!/usr/bin/env sh

cd packages/ciphernode && RUSTFLAGS="-A warnings" cargo run --bin test_encryptor -- $@ 
