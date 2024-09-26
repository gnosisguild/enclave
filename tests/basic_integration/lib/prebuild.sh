#!/usr/bin/env sh

cd packages/ciphernode && RUSTFLAGS="-A warnings" cargo build --bin test_encryptor --bin node --bin aggregator;
