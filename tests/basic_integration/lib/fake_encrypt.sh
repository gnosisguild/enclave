#!/usr/bin/env bash

cd packages/ciphernode && RUSTFLAGS="-A warnings" cargo run --bin test_encryptor -- "$@" 
